use std::collections::HashMap;
use std::option::NoneError;
use std::io;

pub enum Entry {
    Str(String),
    Int(i64),
    List(Vec<Entry>),
    Dictionary(HashMap<String, Entry>)
    
}

fn next(input: &str) -> Result<char, NoneError> {
    Ok(input.chars().next()?)
}

fn until<T: Fn(char) -> bool>(input: &mut &str, test: &T) -> Result<String, NoneError> {
    let mut res = String::new();

    loop {
        let next_char = next(*input)?;

        if !test(next_char) {
            break;
        }        

        res += &next_char.to_string();
        *input = &input[1..];
    }

    Ok(res)
}

fn decode_num<T: Fn(char) -> bool>(input: &mut &str, test: &T) -> Result<i64, NoneError> {
    let num_string = until(input, test)?;
    let r = num_string.parse::<i64>();
    
    match r {
        Ok(v) => Ok(v),
        Err(_e) => Err(NoneError)
    }
}

fn decode_int(input: &mut &str) -> Result<Entry, NoneError> {
    let val = decode_num(input, &|i| i == 'e')?;
    Ok(Entry::Int(val))
}

fn decode_list(input: &str) -> Result<Entry, NoneError> {
    Ok(Entry::Int(0))
}

fn decode_dict(input: &str) -> Result<Entry, NoneError> {
    Ok(Entry::Int(0))
}

pub fn decode(input: &str) -> Result<Entry, NoneError> {
    let id = next(input)?;
    let mut input = &input[1..];
    match id {
        'i' => decode_int(&mut input),
        'l' => decode_list(input),
        'd' => decode_dict(input),
        _ => {
            //It's a byte string
            let str_len = decode_num(&mut input, &|i| i == ':');
            Ok(Entry::Str("Hello".to_string()))
        }
    }
}
