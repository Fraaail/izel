use clap::Parser;
use izel_session::{Session, SessionOptions};
use anyhow::Result;

fn main() -> Result<()> {
    let options = SessionOptions::parse();
    let session = Session::new(options);

    println!("⬡ Izel Compiler (izelc) — Foundation Scaffolding Complete.");
    println!("Creator: @VoxDroid <izeno.contact@gmail.com>");
    println!("Repository: https://github.com/VoxDroid/izel\n");

    let source = std::fs::read_to_string(&session.options.input)?;
    let source_id = izel_span::SourceId(0);
    let mut lexer = izel_lexer::Lexer::new(&source, source_id);

    println!("Lexing file: {:?}", session.options.input);
    loop {
        let token = lexer.next_token();
        println!("Token: {:?} at {:?}", token.kind, token.span);
        if token.kind == izel_lexer::TokenKind::Eof {
            break;
        }
    }
    
    Ok(())
}
