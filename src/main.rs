use std::io::Read;

fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    let mut string = String::new();
    let _ = reqwest::blocking::get("http://www.baidu.com/")?.read_to_string(&mut string)?;
    println!("{}", string);
    Ok(())
}
