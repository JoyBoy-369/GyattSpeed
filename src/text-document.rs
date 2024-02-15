
pub struct TextDocument {
    length: usize,
    line_count: usize,
    line_buffer: Vec<usize>
}

impl TextDocument {
    fn new(file_name: &str) -> Self {
        if let Ok(lines)=read_lines(file_name){

    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


