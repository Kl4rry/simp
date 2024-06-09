fn main() {
    let cmd = simp::get_clap_command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer).unwrap();
    std::fs::write("../simp.1", buffer).unwrap();
}
