pub struct StatusColumn {
    pub active_line: usize,
    pub total_lines: usize,
}

impl StatusColumn {
    pub fn new(active_line: Option<usize>, total_lines: Option<usize>) -> Self {
        StatusColumn {
            active_line: active_line.unwrap_or(0),
            total_lines: total_lines.unwrap_or(0),
        }
    }

    pub fn move_active_line(&mut self, current_line_in_buffer: usize) {
        self.active_line = current_line_in_buffer
    }
}
