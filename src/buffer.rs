pub struct Buffer {
    pub lines: usize,
    pub active_line: usize,
    pub buffer_name: String,
    pub buffer_text: Vec<String>,
}

impl Buffer {
    pub fn new(
        lines: usize,
        active_line: usize,
        buffer_name: String,
        buffer_text: Vec<String>,
    ) -> Self {
        Buffer {
            lines,
            active_line,
            buffer_name,
            buffer_text,
        }
    }
}
