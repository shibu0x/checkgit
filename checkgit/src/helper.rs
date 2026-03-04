use std::io::{Write, stdout};

pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    stdout().flush().unwrap();
}

pub fn move_cursor_up(lines: u16) {
    print!("\x1B[{}A", lines);
}

pub fn move_cursor_right(cols: u16) {
    print!("\x1B[{}C", cols);
}