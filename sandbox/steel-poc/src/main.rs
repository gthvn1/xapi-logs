use steel::steel_vm::engine::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut steel_engine = Engine::new();
    let answer = steel_engine.run("(+ 3 21 13)")?;

    dbg!(answer);

    Ok(())
}
