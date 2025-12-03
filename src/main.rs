use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token {
    IncrementPointer, // >
    DecrementPointer, // <
    IncrementData,    // +
    DecrementData,    // -
    Output,           // .
    Input,            // ,
    LoopStart,        // [
    LoopEnd,          // ]
}

pub fn tokenize(code: &str) -> Vec<Token> {
    code.chars()
        .filter_map(|c| match c {
            '>' => Some(Token::IncrementPointer),
            '<' => Some(Token::DecrementPointer),
            '+' => Some(Token::IncrementData),
            '-' => Some(Token::DecrementData),
            '.' => Some(Token::Output),
            ',' => Some(Token::Input),
            '[' => Some(Token::LoopStart),
            ']' => Some(Token::LoopEnd),
            _ => None,
        })
        .collect()
}

pub type JumpTable = HashMap<usize, usize>;

pub fn parse_loops(tokens: &[Token]) -> Result<JumpTable, String> {
    let mut jump_table: JumpTable = HashMap::new();
    let mut loop_stack: Vec<usize> = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::LoopStart => {
                loop_stack.push(i);
            }
            Token::LoopEnd => {
                if let Some(start_index) = loop_stack.pop() {
                    jump_table.insert(start_index, i);
                    jump_table.insert(i, start_index);
                } else {
                    return Err(format!("Unmatched ']' at index {}", i));
                }
            }
            _ => {}
        }
    }

    if loop_stack.is_empty() {
        Ok(jump_table)
    } else {
        Err(format!("Unmatched '[' at index(es): {:?}", loop_stack))
    }
}

pub struct Interpreter {
    memory: Vec<u8>,
    data_pointer: usize,
    instruction_pointer: usize,
    jump_table: JumpTable,
    tokens: Vec<Token>,
}

impl Interpreter {
    const MEMORY_SIZE: usize = 30000;

    pub fn new(tokens: Vec<Token>, jump_table: JumpTable) -> Self {
        Self {
            memory: vec![0; Self::MEMORY_SIZE],
            data_pointer: Self::MEMORY_SIZE / 2,
            instruction_pointer: 0,
            jump_table,
            tokens,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let tokens_len = self.tokens.len();

        while self.instruction_pointer < tokens_len {
            let token = self.tokens[self.instruction_pointer];

            match token {
                Token::IncrementPointer => {
                    self.data_pointer += 1;
                    if self.data_pointer >= Self::MEMORY_SIZE {
                        return Err("Data pointer out of bounds (right)".to_string());
                    }
                }
                Token::DecrementPointer => {
                    if self.data_pointer == 0 {
                        return Err("Data pointer out of bounds (left)".to_string());
                    }
                    self.data_pointer -= 1;
                }
                Token::IncrementData => {
                    self.memory[self.data_pointer] = self.memory[self.data_pointer].wrapping_add(1);
                }
                Token::DecrementData => {
                    self.memory[self.data_pointer] = self.memory[self.data_pointer].wrapping_sub(1);
                }
                Token::Output => {
                    print!("{}", self.memory[self.data_pointer] as char);
                }
                Token::Input => {
                    let mut buffer = [0u8];
                    if io::stdin().read_exact(&mut buffer).is_ok() {
                        self.memory[self.data_pointer] = buffer[0];
                    } else {
                        self.memory[self.data_pointer] = 0;
                    }
                }
                Token::LoopStart => {
                    if self.memory[self.data_pointer] == 0 {
                        self.instruction_pointer = *self
                            .jump_table
                            .get(&self.instruction_pointer)
                            .ok_or_else(|| {
                                format!(
                                    "Jump table missing entry for '[' at {}",
                                    self.instruction_pointer
                                )
                            })?;
                    }
                }
                Token::LoopEnd => {
                    if self.memory[self.data_pointer] != 0 {
                        self.instruction_pointer = *self
                            .jump_table
                            .get(&self.instruction_pointer)
                            .ok_or_else(|| {
                                format!(
                                    "Jump table missing entry for ']' at {}",
                                    self.instruction_pointer
                                )
                            })?;
                    }
                }
            }

            self.instruction_pointer += 1;
        }

        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <brainfuck_file.bf>", args[0]);
        return;
    }

    let filename = &args[1];

    let bf_code = match fs::read_to_string(filename) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let tokens = tokenize(&bf_code);

    let jump_table = match parse_loops(&tokens) {
        Ok(table) => table,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let mut interpreter = Interpreter::new(tokens, jump_table);

    match interpreter.run() {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta;

    #[test]
    fn test_tokenize_basic() {
        let code = "+-><.,[]";
        let tokens = tokenize(code);

        insta::assert_debug_snapshot!(
            tokens,
            @r"
        [
            IncrementData,
            DecrementData,
            IncrementPointer,
            DecrementPointer,
            Output,
            Input,
            LoopStart,
            LoopEnd,
        ]
        "
        );
    }

    #[test]
    fn test_tokenize_with_comments() {
        let code = "++ Hello World! [<]";
        let tokens = tokenize(code);

        insta::assert_debug_snapshot!(
            tokens,
            @r"
        [
            IncrementData,
            IncrementData,
            LoopStart,
            DecrementPointer,
            LoopEnd,
        ]
        "
        );
    }

    #[test]
    fn test_tokenize_empty() {
        let code = "";
        let tokens = tokenize(code);

        insta::assert_debug_snapshot!(tokens, @"[]");
    }

    #[test]
    fn test_parse_loops_unmatched_loop_start() {
        let tokens = tokenize("[<>]++[");
        let result = parse_loops(&tokens);

        insta::assert_debug_snapshot!(result, @r#"
        Err(
            "Unmatched '[' at index(es): [6]",
        )
        "#);
    }

    #[test]
    fn test_parse_loops_unmatched_loop_end() {
        let tokens = tokenize("[<>]++[]]");
        let result = parse_loops(&tokens);

        insta::assert_debug_snapshot!(result, @r#"
        Err(
            "Unmatched ']' at index 8",
        )
        "#);
    }
}
