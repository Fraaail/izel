use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use izel_fmt::format_source;
use izel_lexer::{Lexer, Token, TokenKind};
use izel_typeck::type_system::{BuiltinWitness, Effect, EffectSet, PrimType, Type};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeclKind {
    Function,
    Type,
    Module,
    Variable,
    Macro,
    Alias,
}

#[derive(Debug, Clone)]
struct SymbolOccurrence {
    name: String,
    span: izel_span::Span,
    range: Range,
    is_definition: bool,
    decl_kind: Option<DeclKind>,
    type_info: Option<String>,
}

#[derive(Debug, Clone)]
struct DocumentAnalysis {
    uri: Url,
    symbols: Vec<SymbolOccurrence>,
}

#[derive(Debug)]
pub struct Backend {
    client: Option<Client>,
    documents: Arc<tokio::sync::RwLock<HashMap<Url, String>>>,
    workspace_roots: Arc<tokio::sync::RwLock<Vec<PathBuf>>>,
}

impl Backend {
    pub fn new(client: Option<Client>) -> Self {
        Self {
            client,
            documents: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            workspace_roots: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    fn semantic_legend() -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: vec![
                SemanticTokenType::KEYWORD,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::TYPE,
                SemanticTokenType::STRING,
                SemanticTokenType::NUMBER,
                SemanticTokenType::COMMENT,
                SemanticTokenType::OPERATOR,
            ],
            token_modifiers: vec![],
        }
    }

    async fn upsert_document(&self, uri: Url, source: String) {
        self.documents.write().await.insert(uri, source);
    }

    async fn remove_document(&self, uri: &Url) {
        self.documents.write().await.remove(uri);
    }

    async fn get_document(&self, uri: &Url) -> Option<String> {
        self.documents.read().await.get(uri).cloned()
    }

    async fn set_workspace_roots(&self, params: &InitializeParams) {
        let roots = Self::extract_workspace_roots(params);
        *self.workspace_roots.write().await = roots;
    }

    fn extract_workspace_roots(params: &InitializeParams) -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Some(folders) = &params.workspace_folders {
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    roots.push(path);
                }
            }
        }

        if let Some(root_uri) = &params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                roots.push(path);
            }
        }

        roots.sort();
        roots.dedup();
        roots
    }

    fn should_skip_workspace_dir(path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };

        matches!(
            name,
            ".git" | ".github" | ".idea" | ".vscode" | "target" | "node_modules" | "dist" | "build"
        )
    }

    fn collect_workspace_iz_files(root: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        const MAX_DEPTH: usize = 16;
        const MAX_FILES: usize = 4000;

        if depth > MAX_DEPTH || out.len() >= MAX_FILES {
            return;
        }

        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            if out.len() >= MAX_FILES {
                break;
            }

            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() {
                if Self::should_skip_workspace_dir(&path) {
                    continue;
                }
                Self::collect_workspace_iz_files(&path, out, depth + 1);
                continue;
            }

            if file_type.is_file() && path.extension().and_then(|s| s.to_str()) == Some("iz") {
                out.push(path);
            }
        }
    }

    async fn collect_all_documents(&self, active_uri: Option<&Url>) -> Vec<(Url, String)> {
        let mut merged = self.documents.read().await.clone();
        let workspace_roots = self.workspace_roots.read().await.clone();

        let mut workspace_files = Vec::new();
        for root in &workspace_roots {
            Self::collect_workspace_iz_files(root, &mut workspace_files, 0);
        }

        for path in workspace_files {
            let Ok(uri) = Url::from_file_path(&path) else {
                continue;
            };
            if merged.contains_key(&uri) {
                continue;
            }
            if let Ok(source) = std::fs::read_to_string(&path) {
                merged.insert(uri, source);
            }
        }

        if let Some(uri) = active_uri {
            if !merged.contains_key(uri) {
                if let Ok(path) = uri.to_file_path() {
                    if let Ok(source) = std::fs::read_to_string(path) {
                        merged.insert(uri.clone(), source);
                    }
                }
            }
        }

        merged.into_iter().collect()
    }

    fn byte_to_position(source: &str, byte_index: usize) -> Position {
        let mut line = 0u32;
        let mut col = 0u32;

        for (idx, ch) in source.char_indices() {
            if idx >= byte_index {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        Position::new(line, col)
    }

    fn range_from_bytes(source: &str, start: usize, end: usize) -> Range {
        let capped_start = start.min(source.len());
        let capped_end = end.min(source.len()).max(capped_start);
        Range::new(
            Self::byte_to_position(source, capped_start),
            Self::byte_to_position(source, capped_end),
        )
    }

    fn fallback_range(source: &str) -> Range {
        if source.is_empty() {
            return Range::default();
        }

        Range::new(
            Position::new(0, 0),
            Self::byte_to_position(source, source.len()),
        )
    }

    fn full_document_range(source: &str) -> Range {
        Self::fallback_range(source)
    }

    fn position_lt(a: Position, b: Position) -> bool {
        (a.line, a.character) < (b.line, b.character)
    }

    fn position_leq(a: Position, b: Position) -> bool {
        (a.line, a.character) <= (b.line, b.character)
    }

    fn range_contains_position(range: Range, pos: Position) -> bool {
        Self::position_leq(range.start, pos) && Self::position_lt(pos, range.end)
    }

    fn ranges_overlap(a: Range, b: Range) -> bool {
        Self::position_lt(a.start, b.end) && Self::position_lt(b.start, a.end)
    }

    fn source_slice<'a>(source: &'a str, token: &Token) -> &'a str {
        let lo = (token.span.lo.0 as usize).min(source.len());
        let hi = (token.span.hi.0 as usize).min(source.len());
        if lo >= hi {
            return "";
        }
        &source[lo..hi]
    }

    fn lex_tokens(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, izel_span::SourceId(1));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let kind = token.kind;
            tokens.push(token);
            if kind == TokenKind::Eof {
                break;
            }
        }
        tokens
    }

    fn is_keyword_token(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Forge
                | TokenKind::Shape
                | TokenKind::Scroll
                | TokenKind::Weave
                | TokenKind::Ward
                | TokenKind::Macro
                | TokenKind::Echo
                | TokenKind::Branch
                | TokenKind::Given
                | TokenKind::Else
                | TokenKind::Loop
                | TokenKind::Each
                | TokenKind::While
                | TokenKind::Break
                | TokenKind::Next
                | TokenKind::Give
                | TokenKind::Let
                | TokenKind::Raw
                | TokenKind::Bridge
                | TokenKind::Flow
                | TokenKind::Tide
                | TokenKind::Zone
                | TokenKind::Dual
                | TokenKind::Seek
                | TokenKind::Catch
                | TokenKind::Draw
                | TokenKind::Open
                | TokenKind::Hidden
                | TokenKind::Pkg
                | TokenKind::Pure
                | TokenKind::Sole
                | TokenKind::SelfKw
                | TokenKind::SelfType
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Nil
                | TokenKind::As
                | TokenKind::In
                | TokenKind::Of
                | TokenKind::Is
                | TokenKind::Not
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Comptime
                | TokenKind::Static
                | TokenKind::Extern
                | TokenKind::Type
                | TokenKind::Alias
                | TokenKind::Impl
                | TokenKind::For
                | TokenKind::Bind
        )
    }

    fn is_operator_token(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Tilde
                | TokenKind::Bang
                | TokenKind::At
                | TokenKind::Pipe
                | TokenKind::Bar
                | TokenKind::DoubleColon
                | TokenKind::Arrow
                | TokenKind::FatArrow
                | TokenKind::DotDot
                | TokenKind::DotDotEq
                | TokenKind::Dot
                | TokenKind::Question
                | TokenKind::QuestionQuestion
                | TokenKind::Pound
                | TokenKind::Equal
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Caret
                | TokenKind::Ampersand
                | TokenKind::AmpersandTilde
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::Le
                | TokenKind::Ge
                | TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::OpenParen
                | TokenKind::CloseParen
                | TokenKind::OpenBrace
                | TokenKind::CloseBrace
                | TokenKind::OpenBracket
                | TokenKind::CloseBracket
                | TokenKind::Comma
                | TokenKind::Semicolon
                | TokenKind::Colon
        )
    }

    fn is_significant_token(kind: TokenKind) -> bool {
        !matches!(kind, TokenKind::Whitespace | TokenKind::Comment)
    }

    fn previous_significant_kind(tokens: &[Token], idx: usize) -> Option<TokenKind> {
        for i in (0..idx).rev() {
            let kind = tokens[i].kind;
            if Self::is_significant_token(kind) {
                return Some(kind);
            }
        }
        None
    }

    fn declaration_kind_from_prev(prev_kind: Option<TokenKind>) -> Option<DeclKind> {
        match prev_kind {
            Some(TokenKind::Forge) => Some(DeclKind::Function),
            Some(TokenKind::Shape)
            | Some(TokenKind::Scroll)
            | Some(TokenKind::Weave)
            | Some(TokenKind::Type)
            | Some(TokenKind::Impl)
            | Some(TokenKind::For) => Some(DeclKind::Type),
            Some(TokenKind::Ward) => Some(DeclKind::Module),
            Some(TokenKind::Let) => Some(DeclKind::Variable),
            Some(TokenKind::Macro) => Some(DeclKind::Macro),
            Some(TokenKind::Alias) => Some(DeclKind::Alias),
            _ => None,
        }
    }

    fn completion_kind_for_decl(decl_kind: DeclKind) -> CompletionItemKind {
        match decl_kind {
            DeclKind::Function => CompletionItemKind::FUNCTION,
            DeclKind::Type => CompletionItemKind::CLASS,
            DeclKind::Module => CompletionItemKind::MODULE,
            DeclKind::Variable => CompletionItemKind::VARIABLE,
            DeclKind::Macro => CompletionItemKind::SNIPPET,
            DeclKind::Alias => CompletionItemKind::TYPE_PARAMETER,
        }
    }

    fn decl_kind_label(decl_kind: Option<DeclKind>) -> &'static str {
        match decl_kind {
            Some(DeclKind::Function) => "forge",
            Some(DeclKind::Type) => "type",
            Some(DeclKind::Module) => "ward",
            Some(DeclKind::Variable) => "binding",
            Some(DeclKind::Macro) => "macro",
            Some(DeclKind::Alias) => "alias",
            None => "symbol",
        }
    }

    fn format_effect(effect: &Effect) -> String {
        match effect {
            Effect::IO => "io".to_string(),
            Effect::Net => "net".to_string(),
            Effect::Alloc => "alloc".to_string(),
            Effect::Panic => "panic".to_string(),
            Effect::Unsafe => "unsafe".to_string(),
            Effect::Time => "time".to_string(),
            Effect::Rand => "rand".to_string(),
            Effect::Env => "env".to_string(),
            Effect::Ffi => "ffi".to_string(),
            Effect::Thread => "thread".to_string(),
            Effect::Mut => "mut".to_string(),
            Effect::Pure => "pure".to_string(),
            Effect::User(name) => name.clone(),
        }
    }

    fn format_effect_set(effects: &EffectSet) -> String {
        match effects {
            EffectSet::Concrete(items) => {
                if items.is_empty() {
                    "pure".to_string()
                } else {
                    items
                        .iter()
                        .map(Self::format_effect)
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            }
            EffectSet::Var(id) => format!("effect_var#{id}"),
            EffectSet::Row(items, tail) => {
                let head = items
                    .iter()
                    .map(Self::format_effect)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head} | {}", Self::format_effect_set(tail))
            }
            EffectSet::Param(name) => name.clone(),
        }
    }

    fn format_type(ty: &Type) -> String {
        match ty {
            Type::Prim(kind) => match kind {
                PrimType::I8 => "i8".to_string(),
                PrimType::I16 => "i16".to_string(),
                PrimType::I32 => "i32".to_string(),
                PrimType::I64 => "i64".to_string(),
                PrimType::I128 => "i128".to_string(),
                PrimType::U8 => "u8".to_string(),
                PrimType::U16 => "u16".to_string(),
                PrimType::U32 => "u32".to_string(),
                PrimType::U64 => "u64".to_string(),
                PrimType::U128 => "u128".to_string(),
                PrimType::F32 => "f32".to_string(),
                PrimType::F64 => "f64".to_string(),
                PrimType::Bool => "bool".to_string(),
                PrimType::Str => "str".to_string(),
                PrimType::Void => "void".to_string(),
                PrimType::None => "none".to_string(),
                PrimType::Never => "never".to_string(),
                PrimType::ZoneAllocator => "zone_alloc".to_string(),
            },
            Type::Adt(def_id) => format!("adt#{}", def_id.0),
            Type::Optional(inner) => format!("?{}", Self::format_type(inner)),
            Type::Cascade(inner) => format!("{}!", Self::format_type(inner)),
            Type::Pointer(inner, is_mut, _) => {
                if *is_mut {
                    format!("*~{}", Self::format_type(inner))
                } else {
                    format!("*{}", Self::format_type(inner))
                }
            }
            Type::Function {
                params,
                ret,
                effects,
            } => {
                let params = params
                    .iter()
                    .map(Self::format_type)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "forge({params}) -> {} !{}",
                    Self::format_type(ret),
                    Self::format_effect_set(effects)
                )
            }
            Type::Var(id) => format!("T#{id}"),
            Type::Param(name) => name.clone(),
            Type::Static(fields) => {
                let fields = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", Self::format_type(ty)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({fields})")
            }
            Type::Assoc(base, name) => format!("{}::{name}", Self::format_type(base)),
            Type::Witness(inner) => format!("Witness<{}>", Self::format_type(inner)),
            Type::BuiltinWitness(kind, inner) => {
                let witness = match kind {
                    BuiltinWitness::NonZero => "NonZero",
                    BuiltinWitness::InBounds => "InBounds",
                    BuiltinWitness::Sorted => "Sorted",
                };
                format!("{witness}<{}>", Self::format_type(inner))
            }
            Type::Predicate(_) => "predicate".to_string(),
            Type::Error => "<type-error>".to_string(),
        }
    }

    fn symbol_occurrences(source: &str) -> Vec<SymbolOccurrence> {
        let tokens = Self::lex_tokens(source);
        let mut out = Vec::new();

        for (idx, token) in tokens.iter().enumerate() {
            if token.kind != TokenKind::Ident {
                continue;
            }

            let name = Self::source_slice(source, token).to_string();
            if name.is_empty() {
                continue;
            }

            let prev_kind = Self::previous_significant_kind(&tokens, idx);
            let decl_kind = Self::declaration_kind_from_prev(prev_kind);
            let is_definition = decl_kind.is_some();
            let range =
                Self::range_from_bytes(source, token.span.lo.0 as usize, token.span.hi.0 as usize);

            out.push(SymbolOccurrence {
                name,
                span: token.span,
                range,
                is_definition,
                decl_kind,
                type_info: None,
            });
        }

        out
    }

    fn uri_label(uri: &Url) -> String {
        if let Ok(path) = uri.to_file_path() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                return name.to_string();
            }
            return path.display().to_string();
        }
        uri.to_string()
    }

    fn analyze_document(uri: Url, source: String) -> DocumentAnalysis {
        let tokens = Self::lex_tokens(&source);
        let mut parser = izel_parser::Parser::new(tokens, source.clone());
        let cst = parser.parse_source_file();

        let base_path = uri
            .to_file_path()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));
        let mut resolver = izel_resolve::Resolver::new(base_path);
        resolver.resolve_source_file(&cst, &source);

        let ast_lowerer = izel_ast_lower::Lowerer::new(&source);
        let ast = ast_lowerer.lower_module(&cst);

        let mut typeck = izel_typeck::TypeChecker::with_builtins();
        typeck.span_to_def = resolver.def_ids.clone();

        let mut ast_modules = std::collections::HashMap::new();
        let loaded_csts = resolver.loaded_csts.read().unwrap();
        for (name, (loaded_cst, loaded_source)) in loaded_csts.iter() {
            let mod_ast_lowerer = izel_ast_lower::Lowerer::new(loaded_source);
            let mod_ast = mod_ast_lowerer.lower_module(loaded_cst);
            ast_modules.insert(name.clone(), mod_ast);
        }
        drop(loaded_csts);

        typeck.check_project(&ast, ast_modules);

        let mut symbols = Self::symbol_occurrences(&source);
        let span_to_def = resolver.def_ids.read().unwrap();
        for symbol in &mut symbols {
            if let Some(def_id) = span_to_def.get(&symbol.span) {
                if let Some(ty) = typeck.def_types.get(def_id) {
                    symbol.type_info = Some(Self::format_type(ty));
                }
            }
        }

        DocumentAnalysis { uri, symbols }
    }

    async fn collect_analyzed_documents(&self, active_uri: Option<&Url>) -> Vec<DocumentAnalysis> {
        self.collect_all_documents(active_uri)
            .await
            .into_iter()
            .map(|(uri, source)| Self::analyze_document(uri, source))
            .collect()
    }

    fn symbol_at_position(
        document: &DocumentAnalysis,
        position: Position,
    ) -> Option<SymbolOccurrence> {
        let occurrences = &document.symbols;

        if let Some(found) = occurrences
            .iter()
            .find(|occ| Self::range_contains_position(occ.range, position))
        {
            return Some(found.clone());
        }

        if position.character == 0 {
            return None;
        }

        let mut previous_position = position;
        previous_position.character -= 1;
        occurrences
            .iter()
            .find(|occ| Self::range_contains_position(occ.range, previous_position))
            .cloned()
    }

    fn find_definitions(
        documents: &[DocumentAnalysis],
        name: &str,
    ) -> Vec<(Url, SymbolOccurrence)> {
        let mut defs = Vec::new();
        for document in documents {
            for symbol in document
                .symbols
                .iter()
                .filter(|occ| occ.name == name && occ.is_definition)
            {
                defs.push((document.uri.clone(), symbol.clone()));
            }
        }

        if !defs.is_empty() {
            return defs;
        }

        for document in documents {
            if let Some(symbol) = document.symbols.iter().find(|occ| occ.name == name) {
                return vec![(document.uri.clone(), symbol.clone())];
            }
        }

        Vec::new()
    }

    fn find_references(
        documents: &[DocumentAnalysis],
        name: &str,
        include_declaration: bool,
    ) -> Vec<(Url, SymbolOccurrence)> {
        let mut refs = Vec::new();
        for document in documents {
            for symbol in document
                .symbols
                .iter()
                .filter(|occ| occ.name == name && (include_declaration || !occ.is_definition))
            {
                refs.push((document.uri.clone(), symbol.clone()));
            }
        }
        refs
    }

    fn semantic_token_type_index(kind: TokenKind, prev_kind: Option<TokenKind>) -> Option<u32> {
        if Self::is_keyword_token(kind) {
            return Some(0);
        }

        match kind {
            TokenKind::Ident => match prev_kind {
                Some(TokenKind::Forge) => Some(2),
                Some(TokenKind::Shape)
                | Some(TokenKind::Scroll)
                | Some(TokenKind::Weave)
                | Some(TokenKind::Type)
                | Some(TokenKind::Alias)
                | Some(TokenKind::Impl)
                | Some(TokenKind::For) => Some(3),
                _ => Some(1),
            },
            TokenKind::Str { .. }
            | TokenKind::InterpolatedStr { .. }
            | TokenKind::ByteStr { .. }
            | TokenKind::Char { .. }
            | TokenKind::Byte { .. } => Some(4),
            TokenKind::Int { .. } | TokenKind::Float => Some(5),
            TokenKind::Comment => Some(6),
            _ if Self::is_operator_token(kind) => Some(7),
            _ => None,
        }
    }

    fn build_semantic_tokens(source: &str, requested_range: Option<Range>) -> Vec<SemanticToken> {
        let tokens = Self::lex_tokens(source);
        let mut absolute_tokens = Vec::<(u32, u32, u32, u32)>::new();

        for (idx, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::Whitespace
                || token.kind == TokenKind::Eof
                || token.kind == TokenKind::Unknown
            {
                continue;
            }

            let prev_kind = Self::previous_significant_kind(&tokens, idx);
            let Some(token_type) = Self::semantic_token_type_index(token.kind, prev_kind) else {
                continue;
            };

            let token_range =
                Self::range_from_bytes(source, token.span.lo.0 as usize, token.span.hi.0 as usize);

            if let Some(range) = requested_range {
                if !Self::ranges_overlap(token_range, range) {
                    continue;
                }
            }

            if token_range.start.line != token_range.end.line {
                continue;
            }

            let length = token_range
                .end
                .character
                .saturating_sub(token_range.start.character);
            if length == 0 {
                continue;
            }

            absolute_tokens.push((
                token_range.start.line,
                token_range.start.character,
                length,
                token_type,
            ));
        }

        absolute_tokens.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

        let mut out = Vec::with_capacity(absolute_tokens.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        let mut first = true;

        for (line, start, length, token_type) in absolute_tokens {
            let delta_line = if first { line } else { line - prev_line };
            let delta_start = if first {
                start
            } else if delta_line == 0 {
                start - prev_start
            } else {
                start
            };

            out.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            });

            prev_line = line;
            prev_start = start;
            first = false;
        }

        out
    }

    fn build_completion_items(
        current_document: &DocumentAnalysis,
        documents: &[DocumentAnalysis],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        let keyword_items = [
            ("forge", CompletionItemKind::KEYWORD),
            ("shape", CompletionItemKind::KEYWORD),
            ("scroll", CompletionItemKind::KEYWORD),
            ("weave", CompletionItemKind::KEYWORD),
            ("ward", CompletionItemKind::KEYWORD),
            ("draw", CompletionItemKind::KEYWORD),
            ("let", CompletionItemKind::KEYWORD),
            ("given", CompletionItemKind::KEYWORD),
            ("else", CompletionItemKind::KEYWORD),
            ("while", CompletionItemKind::KEYWORD),
            ("flow", CompletionItemKind::KEYWORD),
            ("tide", CompletionItemKind::KEYWORD),
            ("pure", CompletionItemKind::KEYWORD),
            ("std", CompletionItemKind::MODULE),
        ];

        for (label, kind) in keyword_items {
            if seen.insert(label.to_string()) {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(kind),
                    detail: Some("Izel keyword/module".to_string()),
                    ..Default::default()
                });
            }
        }

        for document in documents {
            for occ in document.symbols.iter().filter(|occ| occ.is_definition) {
                if !seen.insert(occ.name.clone()) {
                    continue;
                }

                let kind = occ
                    .decl_kind
                    .map(Self::completion_kind_for_decl)
                    .or(Some(CompletionItemKind::VARIABLE));

                let origin = if document.uri == current_document.uri {
                    "current document".to_string()
                } else {
                    Self::uri_label(&document.uri)
                };

                let mut detail = format!("{} from {origin}", Self::decl_kind_label(occ.decl_kind));
                if let Some(ty) = &occ.type_info {
                    detail.push_str(&format!(" : {ty}"));
                }

                items.push(CompletionItem {
                    label: occ.name.clone(),
                    kind,
                    detail: Some(detail),
                    documentation: Some(Documentation::String(format!(
                        "Defined in {}",
                        Self::uri_label(&document.uri)
                    ))),
                    sort_text: Some(if document.uri == current_document.uri {
                        format!("0_{}", occ.name)
                    } else {
                        format!("1_{}", occ.name)
                    }),
                    ..Default::default()
                });
            }
        }

        items.sort_by(|a, b| {
            let left = a.sort_text.as_deref().unwrap_or(&a.label);
            let right = b.sort_text.as_deref().unwrap_or(&b.label);
            left.cmp(right)
        });
        items
    }

    fn build_hover_markdown(
        symbol: &SymbolOccurrence,
        definitions: &[(Url, SymbolOccurrence)],
    ) -> String {
        let mut lines = vec![format!("**{}**", symbol.name)];
        lines.push(format!("Kind: {}", Self::decl_kind_label(symbol.decl_kind)));

        if let Some(ty) = &symbol.type_info {
            lines.push(format!("Type: {ty}"));
        }

        if !definitions.is_empty() {
            let listed = definitions
                .iter()
                .take(3)
                .map(|(uri, def)| {
                    format!(
                        "- {}:{}:{}",
                        Self::uri_label(uri),
                        def.range.start.line + 1,
                        def.range.start.character + 1
                    )
                })
                .collect::<Vec<_>>();
            lines.push("Definitions:".to_string());
            lines.extend(listed);

            if definitions.len() > 3 {
                lines.push(format!("- and {} more", definitions.len() - 3));
            }
        }

        lines.join("\n")
    }

    fn build_inlay_hints(source: &str, requested_range: Range) -> Vec<InlayHint> {
        let tokens = Self::lex_tokens(source);
        let mut hints = Vec::new();

        for idx in 0..tokens.len() {
            if tokens[idx].kind != TokenKind::Let {
                continue;
            }

            let ident_idx = ((idx + 1)..tokens.len()).find(|i| {
                let kind = tokens[*i].kind;
                Self::is_significant_token(kind) && kind == TokenKind::Ident
            });

            let Some(ident_idx) = ident_idx else {
                continue;
            };

            let next_kind = ((ident_idx + 1)..tokens.len())
                .map(|i| tokens[i].kind)
                .find(|k| Self::is_significant_token(*k));
            if next_kind == Some(TokenKind::Colon) {
                continue;
            }

            let ident_range = Self::range_from_bytes(
                source,
                tokens[ident_idx].span.lo.0 as usize,
                tokens[ident_idx].span.hi.0 as usize,
            );
            if !Self::range_contains_position(requested_range, ident_range.start) {
                continue;
            }

            hints.push(InlayHint {
                position: ident_range.end,
                label: InlayHintLabel::String(": ?".to_string()),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(InlayHintTooltip::String(
                    "Type hint placeholder until full inlay inference is implemented".to_string(),
                )),
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }

        hints
    }

    fn is_valid_identifier_name(name: &str) -> bool {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };

        if !(first == '_' || first.is_ascii_alphabetic()) {
            return false;
        }

        chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    }

    fn action_kind_allowed(kind: &CodeActionKind, allowed: &Option<Vec<CodeActionKind>>) -> bool {
        match allowed {
            None => true,
            Some(allowed) => allowed
                .iter()
                .any(|requested| kind.as_str().starts_with(requested.as_str())),
        }
    }

    fn build_code_actions_for_document(
        uri: &Url,
        source: &str,
        params: &CodeActionParams,
    ) -> CodeActionResponse {
        let mut actions = Vec::new();

        let formatted = format_source(source);
        if formatted != source
            && Self::action_kind_allowed(&CodeActionKind::SOURCE_FIX_ALL, &params.context.only)
        {
            let mut edits = HashMap::new();
            edits.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Self::full_document_range(source),
                    new_text: formatted,
                }],
            );

            let action = CodeAction {
                title: "Format document".to_string(),
                kind: Some(CodeActionKind::SOURCE_FIX_ALL),
                edit: Some(WorkspaceEdit {
                    changes: Some(edits),
                    ..Default::default()
                }),
                ..Default::default()
            };
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        for diagnostic in &params.context.diagnostics {
            if !diagnostic.message.contains("requires an initializer") {
                continue;
            }

            if !Self::action_kind_allowed(&CodeActionKind::QUICKFIX, &params.context.only) {
                continue;
            }

            let mut edits = HashMap::new();
            edits.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Range::new(diagnostic.range.end, diagnostic.range.end),
                    new_text: " = 0".to_string(),
                }],
            );

            let action = CodeAction {
                title: "Insert placeholder initializer".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(edits),
                    ..Default::default()
                }),
                ..Default::default()
            };
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        actions
    }

    fn build_diagnostics(source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let mut parser = izel_parser::Parser::new(Self::lex_tokens(source), source.to_string());
        let cst = parser.parse_source_file();

        let ast_lowerer = izel_ast_lower::Lowerer::new(source);
        let ast = ast_lowerer.lower_module(&cst);

        let mut typeck = izel_typeck::TypeChecker::new();
        typeck.check_ast(&ast);

        if !typeck.diagnostics.is_empty() {
            for diag in &typeck.diagnostics {
                let mut range = diag
                    .labels
                    .first()
                    .map(|label| Self::range_from_bytes(source, label.range.start, label.range.end))
                    .unwrap_or_else(|| Self::fallback_range(source));

                if range == Range::default() && !source.is_empty() {
                    range = Self::fallback_range(source);
                }

                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: diag.message.clone(),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }

    async fn validate_document(&self, uri: Url, source: String) {
        let diagnostics = Self::build_diagnostics(&source);

        if let Some(client) = &self.client {
            client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }

    async fn formatting_edits_for_uri(&self, uri: &Url) -> Option<Vec<TextEdit>> {
        let source = self.get_document(uri).await?;
        let formatted = format_source(&source);

        if formatted == source {
            return None;
        }

        Some(vec![TextEdit {
            range: Self::full_document_range(&source),
            new_text: formatted,
        }])
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.set_workspace_roots(&params).await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "::".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::SOURCE_FIX_ALL,
                        ]),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    },
                )),
                inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
                    InlayHintOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    },
                ))),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: Self::semantic_legend(),
                            range: Some(true),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        if let Some(client) = &self.client {
            client
                .log_message(MessageType::INFO, "Izel Language Server is initialized!")
                .await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Some(client) = &self.client {
            client
                .log_message(
                    MessageType::INFO,
                    format!("Opened file: {}", params.text_document.uri),
                )
                .await;
        }

        let uri = params.text_document.uri;
        let source = params.text_document.text;
        self.upsert_document(uri.clone(), source.clone()).await;
        self.validate_document(uri, source).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.pop() {
            let uri = params.text_document.uri;
            self.upsert_document(uri.clone(), change.text.clone()).await;
            self.validate_document(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.remove_document(&params.text_document.uri).await;

        if let Some(client) = &self.client {
            client
                .publish_diagnostics(params.text_document.uri, vec![], None)
                .await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let documents = self.collect_analyzed_documents(Some(uri)).await;
        let Some(current_document) = documents.iter().find(|doc| doc.uri == *uri) else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(current_document, position) else {
            return Ok(None);
        };

        let definitions = Self::find_definitions(&documents, &symbol.name);
        let markdown = Self::build_hover_markdown(&symbol, &definitions);

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: Some(symbol.range),
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let documents = self.collect_analyzed_documents(Some(&uri)).await;
        let Some(current_document) = documents.iter().find(|doc| doc.uri == uri) else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(current_document, position) else {
            return Ok(None);
        };

        let locations = Self::find_definitions(&documents, &symbol.name)
            .into_iter()
            .map(|(def_uri, occ)| Location {
                uri: def_uri,
                range: occ.range,
            })
            .collect::<Vec<_>>();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(GotoDefinitionResponse::Array(locations)))
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let documents = self.collect_analyzed_documents(Some(&uri)).await;
        let Some(current_document) = documents.iter().find(|doc| doc.uri == uri) else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(current_document, position) else {
            return Ok(None);
        };

        let locations =
            Self::find_references(&documents, &symbol.name, params.context.include_declaration)
                .into_iter()
                .map(|(ref_uri, occ)| Location {
                    uri: ref_uri,
                    range: occ.range,
                })
                .collect::<Vec<_>>();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;

        let documents = self.collect_analyzed_documents(Some(&uri)).await;
        let Some(current_document) = documents.iter().find(|doc| doc.uri == uri) else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(current_document, params.position) else {
            return Ok(None);
        };

        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: symbol.range,
            placeholder: symbol.name,
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        if !Self::is_valid_identifier_name(&params.new_name) {
            return Err(Error::invalid_params(
                "new_name must be a valid ASCII identifier",
            ));
        }

        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let documents = self.collect_analyzed_documents(Some(&uri)).await;
        let Some(current_document) = documents.iter().find(|doc| doc.uri == uri) else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(current_document, position) else {
            return Ok(None);
        };

        let refs = Self::find_references(&documents, &symbol.name, true);

        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for (ref_uri, occ) in refs {
            changes.entry(ref_uri).or_default().push(TextEdit {
                range: occ.range,
                new_text: params.new_name.clone(),
            });
        }

        let edits_count = changes.values().map(|edits| edits.len()).sum::<usize>();

        if edits_count == 0 {
            return Ok(None);
        }

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;

        let documents = self.collect_analyzed_documents(Some(&uri)).await;
        let Some(current_document) = documents.iter().find(|doc| doc.uri == uri) else {
            return Ok(None);
        };

        let items = Self::build_completion_items(current_document, &documents);

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let actions = Self::build_code_actions_for_document(&uri, &source, &params);
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let hints = Self::build_inlay_hints(&source, params.range);
        if hints.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hints))
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: Self::build_semantic_tokens(&source, None),
        })))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: Self::build_semantic_tokens(&source, Some(params.range)),
        })))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        Ok(self
            .formatting_edits_for_uri(&params.text_document.uri)
            .await)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        Ok(self
            .formatting_edits_for_uri(&params.text_document.uri)
            .await)
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(Some(client)));
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn run_server_sync() {
    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(run_server()),
        Err(err) => eprintln!("failed to start izel_lsp runtime: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///tmp/test.iz").expect("valid test URL")
    }

    fn test_uri_named(name: &str) -> Url {
        Url::parse(&format!("file:///tmp/{name}.iz")).expect("valid named test URL")
    }

    fn hover_text(hover: Hover) -> String {
        match hover.contents {
            HoverContents::Markup(markup) => markup.value,
            HoverContents::Scalar(MarkedString::String(text)) => text,
            HoverContents::Scalar(MarkedString::LanguageString(text)) => text.value,
            HoverContents::Array(items) => items
                .into_iter()
                .map(|item| match item {
                    MarkedString::String(text) => text,
                    MarkedString::LanguageString(text) => text.value,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    fn test_backend() -> Backend {
        Backend::new(None)
    }

    fn formatting_options() -> FormattingOptions {
        FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: HashMap::new(),
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        }
    }

    #[tokio::test]
    async fn initialize_reports_expected_capabilities() {
        let backend = test_backend();
        let init = backend
            .initialize(InitializeParams::default())
            .await
            .expect("initialize should succeed");

        assert!(matches!(
            init.capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL))
        ));
        assert_eq!(
            init.capabilities.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        );
        assert!(init.capabilities.definition_provider.is_some());
        assert!(init.capabilities.references_provider.is_some());
        assert!(init.capabilities.rename_provider.is_some());
        assert!(init.capabilities.code_action_provider.is_some());
        assert!(init.capabilities.inlay_hint_provider.is_some());
        assert!(init.capabilities.semantic_tokens_provider.is_some());
        assert!(init.capabilities.document_formatting_provider.is_some());
        assert!(init
            .capabilities
            .document_range_formatting_provider
            .is_some());

        let trigger_chars = init
            .capabilities
            .completion_provider
            .and_then(|c| c.trigger_characters)
            .unwrap_or_default();
        assert!(trigger_chars.contains(&".".to_string()));
        assert!(trigger_chars.contains(&"::".to_string()));
    }

    #[tokio::test]
    async fn did_open_change_and_close_manage_document_state() {
        let backend = test_backend();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: test_uri(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: "echo { let x }".to_string(),
                },
            })
            .await;

        assert!(backend.get_document(&test_uri()).await.is_some());

        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: test_uri(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "shape Packet {}".to_string(),
                }],
            })
            .await;

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: test_uri() },
            })
            .await;

        assert!(backend.get_document(&test_uri()).await.is_none());
    }

    #[tokio::test]
    async fn hover_definition_references_prepare_rename_and_rename_are_callable() {
        let backend = test_backend();
        let source = "forge add(a: i32) -> i32 { give a }\nforge main() -> i32 { let value = add(1); give value }";
        let uri = test_uri();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: source.to_string(),
                },
            })
            .await;

        let call_offset = source.find("add(1)").expect("call site should exist");
        let call_position = Backend::byte_to_position(source, call_offset + 1);

        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("hover should succeed");
        assert!(hover.is_some());

        let definition = backend
            .goto_definition(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("definition should succeed");
        assert!(definition.is_some());

        let refs = backend
            .references(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: true,
                },
            })
            .await
            .expect("references should succeed")
            .expect("references should be present");
        assert!(refs.len() >= 2);

        let prepared = backend
            .prepare_rename(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: call_position,
            })
            .await
            .expect("prepare rename should succeed");
        assert!(prepared.is_some());

        let rename_edit = backend
            .rename(RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                new_name: "sum".to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("rename should succeed")
            .expect("rename edits should exist");

        let edits = rename_edit
            .changes
            .and_then(|c| c.get(&uri).cloned())
            .unwrap_or_default();
        assert!(edits.len() >= 2);
    }

    #[tokio::test]
    async fn completion_actions_semantic_tokens_inlay_and_formatting_are_callable() {
        let backend = test_backend();
        let source = "forge main() -> i32 { let x = 1+2; give x }";
        let uri = test_uri();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: source.to_string(),
                },
            })
            .await;

        let completion = backend
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .expect("completion should succeed");
        assert!(completion.is_some());

        let code_actions = backend
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                context: CodeActionContext {
                    diagnostics: vec![Diagnostic {
                        range: Range::new(Position::new(0, 10), Position::new(0, 11)),
                        message: "binding requires an initializer".to_string(),
                        ..Default::default()
                    }],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("code action should succeed");
        assert!(code_actions.is_some());

        let semantic_full = backend
            .semantic_tokens_full(SemanticTokensParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await
            .expect("semantic full should succeed");
        assert!(semantic_full.is_some());

        let semantic_range = backend
            .semantic_tokens_range(SemanticTokensRangeParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(0, 40)),
            })
            .await
            .expect("semantic range should succeed");
        assert!(semantic_range.is_some());

        let hints = backend
            .inlay_hint(InlayHintParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(0, 60)),
            })
            .await
            .expect("inlay hints should succeed");
        assert!(hints.is_some());

        let formatted = backend
            .formatting(DocumentFormattingParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                options: formatting_options(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("formatting should succeed");
        assert!(formatted.is_some());

        let range_formatted = backend
            .range_formatting(DocumentRangeFormattingParams {
                text_document: TextDocumentIdentifier { uri },
                range: Range::new(Position::new(0, 0), Position::new(0, 20)),
                options: formatting_options(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("range formatting should succeed");
        assert!(range_formatted.is_some());
    }

    #[tokio::test]
    async fn cross_file_symbol_queries_and_richer_hover_completion_work() {
        let backend = test_backend();
        let def_uri = test_uri_named("defs");
        let use_uri = test_uri_named("uses");
        let def_source = "forge helper() -> i32 { give 1 }";
        let use_source = "forge main() -> i32 { let value = helper(); give value }";

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: def_uri.clone(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: def_source.to_string(),
                },
            })
            .await;

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: use_uri.clone(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: use_source.to_string(),
                },
            })
            .await;

        let call_offset = use_source
            .find("helper()")
            .expect("helper call should exist in use_source");
        let call_position = Backend::byte_to_position(use_source, call_offset + 1);

        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: use_uri.clone(),
                    },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("hover should succeed")
            .expect("hover should return data");
        let hover_text = hover_text(hover);
        assert!(hover_text.contains("helper"));
        assert!(hover_text.contains("Definitions:"));

        let definition = backend
            .goto_definition(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: use_uri.clone(),
                    },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("definition should succeed")
            .expect("definition should return locations");

        let definition_locs = match definition {
            GotoDefinitionResponse::Array(items) => items,
            GotoDefinitionResponse::Scalar(item) => vec![item],
            GotoDefinitionResponse::Link(items) => items
                .into_iter()
                .map(|item| Location {
                    uri: item.target_uri,
                    range: item.target_range,
                })
                .collect(),
        };
        assert!(definition_locs.iter().any(|loc| loc.uri == def_uri));

        let refs = backend
            .references(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: use_uri.clone(),
                    },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: true,
                },
            })
            .await
            .expect("references should succeed")
            .expect("references should return locations");
        assert!(refs.iter().any(|loc| loc.uri == def_uri));
        assert!(refs.iter().any(|loc| loc.uri == use_uri));

        let rename_edit = backend
            .rename(RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: use_uri.clone(),
                    },
                    position: call_position,
                },
                new_name: "helper2".to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("rename should succeed")
            .expect("rename edit should exist");

        let changes = rename_edit.changes.expect("changes should be present");
        assert!(changes.contains_key(&def_uri));
        assert!(changes.contains_key(&use_uri));

        let completion = backend
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: use_uri.clone(),
                    },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .expect("completion should succeed")
            .expect("completion should return items");

        let items = match completion {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items,
        };

        let helper_item = items
            .iter()
            .find(|item| item.label == "helper")
            .expect("helper completion should be present");
        assert!(helper_item
            .detail
            .as_ref()
            .map(|d| d.contains("from"))
            .unwrap_or(false));
    }

    #[test]
    fn build_diagnostics_reports_expected_results() {
        let clean = Backend::build_diagnostics("shape Packet {}");
        assert!(clean.is_empty());

        let bad = Backend::build_diagnostics("echo { let x }");
        assert!(
            bad.iter()
                .any(|d| d.message.contains("requires an initializer")),
            "invalid echo should produce a diagnostic"
        );
        assert!(
            bad.iter().any(|d| d.range != Range::default()),
            "invalid echo should produce mapped source ranges"
        );
    }

    #[tokio::test]
    async fn validate_document_noops_without_client() {
        let backend = test_backend();
        backend
            .validate_document(test_uri(), "shape Ready {}".to_string())
            .await;
    }

    #[tokio::test]
    async fn validate_document_publishes_with_client() {
        let (service, _socket) = LspService::new(|client| Backend::new(Some(client)));

        service
            .inner()
            .validate_document(test_uri(), "echo { let x }".to_string())
            .await;
    }
}
