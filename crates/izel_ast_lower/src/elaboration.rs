use izel_parser::ast;

pub fn elaborate_dual(dual: &mut ast::Dual) -> Option<ast::Item> {
    // 1. Find missing methods.
    let mut encode_fn = None;
    let mut decode_fn = None;
    
    for item in &dual.items {
        if let ast::Item::Forge(f) = item {
            if f.name == "encode" { encode_fn = Some(f.clone()); }
            if f.name == "decode" { decode_fn = Some(f.clone()); }
        }
    }
    
    if let (Some(encode), None) = (&encode_fn, &decode_fn) {
        let decode = derive_decode_from_encode(encode);
        dual.items.push(ast::Item::Forge(decode.clone()));
        decode_fn = Some(decode);
    } else if let (None, Some(decode)) = (&encode_fn, &decode_fn) {
        let encode = derive_encode_from_decode(decode);
        dual.items.push(ast::Item::Forge(encode.clone()));
        encode_fn = Some(encode);
    }

    // 2. Auto-generate test for effectful dual shapes.
    // If either encode or decode has effects (e.g. !io), generate a test.
    if let (Some(encode), Some(decode)) = (&encode_fn, &decode_fn) {
        if !encode.effects.is_empty() || !decode.effects.is_empty() {
            return Some(generate_roundtrip_test(&dual.name, encode, decode));
        }
    }

    None
}

fn generate_roundtrip_test(shape_name: &str, encode: &ast::Forge, _decode: &ast::Forge) -> ast::Item {
    let test_name = format!("test_{}_roundtrip", shape_name.to_lowercase());
    ast::Item::Forge(ast::Forge {
        name: format!("{}_test", shape_name),
        is_flow: false,
        generic_params: encode.generic_params.clone(),
        params: vec![],
        ret_type: ast::Type::Prim("void".into()),
        effects: encode.effects.clone(),
        attributes: vec![ast::Attribute { name: "test".into(), args: vec![], span: encode.span }],
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block { stmts: vec![], expr: Some(Box::new(ast::Expr::Ident("todo".into(), encode.span))), span: encode.span }),
        span: encode.span,
    })
}

fn derive_decode_from_encode(encode: &ast::Forge) -> ast::Forge {
    let mut params = Vec::new();
    // Default signature for derived decode: decode(&self, raw: &JsonValue) -> Result<T>
    params.push(ast::Param { name: "self".into(), ty: ast::Type::Pointer(Box::new(ast::Type::SelfType), false), span: encode.span });
    params.push(ast::Param { name: "raw".into(), ty: ast::Type::Prim("JsonValue".into()), span: encode.span });
    
    // Simplistic inversion for proof of concept
    ast::Forge {
        name: "decode".to_string(),
        is_flow: false,
        generic_params: encode.generic_params.clone(),
        params,
        ret_type: ast::Type::Cascade(Box::new(ast::Type::Prim("T".into()))),
        effects: encode.effects.clone(),
        attributes: encode.attributes.clone(),
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block { stmts: vec![], expr: Some(Box::new(ast::Expr::Ident("todo".into(), encode.span))), span: encode.span }),
        span: encode.span,
    }
}

fn derive_encode_from_decode(decode: &ast::Forge) -> ast::Forge {
    // Similarly simplistic output for derived encode...
    let mut params = Vec::new();
    params.push(ast::Param { name: "self".into(), ty: ast::Type::Pointer(Box::new(ast::Type::SelfType), false), span: decode.span });
    params.push(ast::Param { name: "val".into(), ty: ast::Type::Pointer(Box::new(ast::Type::Prim("T".into())), false), span: decode.span });
    
    ast::Forge {
        name: "encode".to_string(),
        is_flow: false,
        generic_params: decode.generic_params.clone(),
        params,
        ret_type: ast::Type::Prim("JsonValue".into()),
        effects: decode.effects.clone(),
        attributes: decode.attributes.clone(),
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block { stmts: vec![], expr: Some(Box::new(ast::Expr::Ident("todo".into(), decode.span))), span: decode.span }),
        span: decode.span,
    }
}
