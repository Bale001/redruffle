use codegen::codegen::ActionBlock;
use parser::{ast::expression::Symbol, parser::ParseError, Parser};
use ruffle_core::avm1::activation::{Activation, ActivationIdentifier};
use ruffle_core::backend::{
    audio::NullAudioBackend, locale::NullLocaleBackend, log::LogBackend,
    navigator::NullNavigatorBackend, render::NullRenderer, storage::MemoryStorageBackend,
    ui::NullUiBackend, video::NullVideoBackend,
};
use ruffle_core::tag_utils::{SwfMovie, SwfSlice};
use ruffle_core::Player;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

fn compile(src: String) -> Result<Vec<u8>, ParseError> {
    let mut parser = Parser::new(&src);
    let mut instructions = Vec::new();
    while let Some(instr) = parser.parse_instruction(true)? {
        instructions.push(instr);
    }
    let defs = parser.definitions();
    let mut gen_block = ActionBlock::new(None, None);
    for (name, function) in defs.iter() {
        if let Symbol::Function(params, code, _, flags) = function {
            gen_block
                .write_function(Some(name.base()), params, code, *flags)
                .unwrap();
        }
    }
    for instr in instructions {
        gen_block.write_instruction(&instr).unwrap();
    }
    Ok(gen_block.into_bytes())
}

struct RedLogBackend();

impl RedLogBackend {
    fn new() -> Self {
        Self()
    }
}

impl LogBackend for RedLogBackend {
    fn avm_trace(&self, message: &str) {
        println!("{}", message);
    }
}

fn main() {
    let player = Player::new(
        Box::new(NullRenderer::new()),
        Box::new(NullAudioBackend::new()),
        Box::new(NullNavigatorBackend::new()),
        Box::new(MemoryStorageBackend::default()),
        Box::new(NullLocaleBackend::new()),
        Box::new(NullVideoBackend::new()),
        Box::new(RedLogBackend::new()),
        Box::new(NullUiBackend::new()),
    )
    .unwrap();
    let mut write = player.lock().unwrap();
    let movie = Arc::new(SwfMovie::empty(32));
    write.set_root_movie(movie);
    write.set_max_execution_duration(Duration::from_secs(300));
    write.update(|context| {
        let mut input_buffer = String::new();
        let mut curly_count: u32 = 0;
        let mut square_count: u32 = 0;
        let mut paren_count: u32 = 0;

        print!(">>> ");
        let mut activation =
            Activation::from_stub(context.reborrow(), ActivationIdentifier::root("top"));
        loop {
            let mut input = String::new();
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut input).unwrap();
            for c in input.chars() {
                match c {
                    '{' => curly_count += 1,
                    '[' => square_count += 1,
                    '(' => paren_count += 1,
                    '}' => curly_count = curly_count.saturating_sub(1),
                    ']' => square_count = square_count.saturating_sub(1),
                    ')' => paren_count = paren_count.saturating_sub(1),
                    _ => (),
                }
            }
            input_buffer.push_str(&input);
            if curly_count == 0 && square_count == 0 && paren_count == 0 {
                match compile(std::mem::take(&mut input_buffer)) {
                    Ok(bytes) => {
                        let mut fake_movie = SwfMovie::empty(32);
                        fake_movie.extend_from_slice(&bytes);
                        let slice = SwfSlice::from(Arc::new(fake_movie));
                        match activation.run_actions(slice) {
                            Ok(_) => (),
                            Err(e) => println!("{}", e),
                        }
                    }
                    Err(e) => println!("{}", e),
                }

                print!(">>> ");
            } else {
                print!("... ");
            }
        }
    });
}
