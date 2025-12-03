use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};

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
        Err(format!("Unmatched '[' at index {}", loop_stack[0]))
    }
}

pub struct Interpreter {
    memory: Vec<u8>,
    data_pointer: usize,
    instruction_pointer: usize,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    const MEMORY_SIZE: usize = 30000;

    pub fn new() -> Self {
        Self {
            memory: vec![0; Self::MEMORY_SIZE],
            data_pointer: Self::MEMORY_SIZE / 2,
            instruction_pointer: 0,
        }
    }

    pub fn run(&mut self, tokens: &[Token], jump_table: &JumpTable) -> Result<(), String> {
        let tokens_len = tokens.len();
        self.instruction_pointer = 0;

        while self.instruction_pointer < tokens_len {
            let token = tokens[self.instruction_pointer];

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
                    io::stdout().flush().map_err(|e| e.to_string())?;
                }
                Token::Input => match io::stdin().bytes().next() {
                    Some(Ok(byte)) => self.memory[self.data_pointer] = byte,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => self.memory[self.data_pointer] = 0,
                },
                Token::LoopStart => {
                    if self.memory[self.data_pointer] == 0 {
                        self.instruction_pointer =
                            *jump_table.get(&self.instruction_pointer).ok_or_else(|| {
                                format!(
                                    "Jump table missing entry for '[' at {}",
                                    self.instruction_pointer
                                )
                            })?;
                    }
                }
                Token::LoopEnd => {
                    if self.memory[self.data_pointer] != 0 {
                        self.instruction_pointer =
                            *jump_table.get(&self.instruction_pointer).ok_or_else(|| {
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

    pub fn print_memory_snapshot(&self, range: usize) {
        let start = self.data_pointer.saturating_sub(range);
        let end = (self.data_pointer + range + 1).min(Self::MEMORY_SIZE);

        print!("Addr:");
        for i in start..end {
            print!("{:>7}", i);
        }
        println!();

        print!("Data:");
        for i in start..end {
            print!("{:>7}", self.memory[i]);
        }
        println!();

        print!("Ptrs:");
        for i in start..end {
            if i == self.data_pointer {
                print!("  ^^^^^");
            } else {
                print!("       ");
            }
        }
        println!();
    }
}

fn run_repl() -> Result<(), String> {
    let mut interpreter = Interpreter::new();

    println!("Brainfuck REPL");
    println!("Type 'exit' to exit, or 'mem' to show memory snapshot.");

    loop {
        print!("> ");
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut input = String::new();

        let bytes_read = io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            println!();
            break;
        }

        let bf_code = input.trim();

        if bf_code.is_empty() {
            continue;
        }

        match bf_code {
            "quit" | "exit" => {
                break;
            }
            "mem" | "memory" => {
                interpreter.print_memory_snapshot(5);
                continue;
            }
            _ => {}
        }

        let tokens = tokenize(bf_code);

        if tokens.is_empty() {
            continue;
        }

        let jump_table = match parse_loops(&tokens) {
            Ok(jump_table) => jump_table,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        match interpreter.run(&tokens, &jump_table) {
            Ok(_) => {
                println!(
                    "Cell[DP={}] = {}",
                    interpreter.data_pointer, interpreter.memory[interpreter.data_pointer]
                );
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }

    Ok(())
}

fn run_file(filename: &str) -> Result<(), String> {
    let bf_code =
        fs::read_to_string(filename).map_err(|e| format!("Cannot read {}: {}", filename, e))?;

    let tokens = tokenize(&bf_code);

    let jump_table = parse_loops(&tokens)?;

    let mut interpreter = Interpreter::new();

    interpreter.run(&tokens, &jump_table)?;
    println!();

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let result = if args.len() > 1 {
        run_file(&args[1])
    } else {
        run_repl()
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
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
            "Unmatched '[' at index 6",
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
