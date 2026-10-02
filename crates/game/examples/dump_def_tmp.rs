fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut o = oag_game::title::open_source(&a[1], Vec::new(), Vec::new()).unwrap();
    let blob = o.archives.read_name(&a[2]).unwrap();
    print!("{}", oag_tables::fexml::text(&blob).unwrap());
}
