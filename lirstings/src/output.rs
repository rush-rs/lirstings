use std::marker::PhantomData;

use crate::renderer;

pub struct OutputWriter<Renderer: renderer::Renderer> {
    line_numbers: Box<dyn Iterator<Item = usize>>,
    output_string: String,
    inline: bool,
    _renderer: PhantomData<Renderer>,
}

impl<Renderer: renderer::Renderer> OutputWriter<Renderer> {
    pub fn new(
        mut line_numbers: impl Iterator<Item = usize> + 'static,
        inline: bool,
        fancyvrb_args: &str,
        label: Option<String>,
    ) -> Self {
        let first_line = line_numbers.next().unwrap_or_default();
        Self {
            line_numbers: Box::new(line_numbers),
            output_string: Renderer::head(inline, label, fancyvrb_args, first_line).into_owned(),
            inline,
            _renderer: PhantomData,
        }
    }

    pub fn push_str(&mut self, str: &str) {
        let lines: Vec<_> = str.split('\n').collect();
        let last_line_index = lines.len() - 1;
        for (index, line) in lines.into_iter().enumerate() {
            self.output_string.push_str(line);
            if index != last_line_index && !self.inline {
                self.output_string.push_str(&Renderer::newline(
                    self.line_numbers.next().unwrap_or_default(),
                ));
            }
        }
    }

    pub fn finish(mut self) -> String {
        self.output_string.push_str(&Renderer::tail(self.inline));
        self.output_string
    }
}
