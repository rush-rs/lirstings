pub struct Output {
    line_numbers: Box<dyn Iterator<Item = usize>>,
    output_string: String,
    inline: bool,
}

const SET_COUNTER_COMMAND: &str = "×setcounter{TS2TeXLineNo}";

impl Output {
    pub fn new(mut line_numbers: impl Iterator<Item = usize> + 'static, inline: bool) -> Self {
        let first_number = line_numbers.next().unwrap_or_default();
        Self {
            line_numbers: Box::new(line_numbers),
            output_string: match inline {
                true => String::new(),
                false => format!("{SET_COUNTER_COMMAND}{{{first_number}}}"),
            },
            inline,
        }
    }

    pub fn push_str(&mut self, str: &str) {
        let lines: Vec<_> = str.split('\n').collect();
        let last_line_index = lines.len() - 1;
        for (index, line) in lines.into_iter().enumerate() {
            self.output_string.push_str(line);
            if index != last_line_index && !self.inline {
                self.output_string.push('\n');
                self.output_string.push_str(&format!(
                    "{SET_COUNTER_COMMAND}{{{}}}",
                    self.line_numbers.next().unwrap_or_default()
                ));
            }
        }
    }

    pub fn push(&mut self, char: char) {
        self.output_string.push(char);
        if char == '\n' && !self.inline {
            self.output_string.push_str(&format!(
                "{SET_COUNTER_COMMAND}{{{}}}",
                self.line_numbers.next().unwrap_or_default()
            ));
        }
    }

    pub fn finish(self) -> String {
        self.output_string
    }
}
