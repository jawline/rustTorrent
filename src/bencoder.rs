use std::collections::HashMap;
use std::option::NoneError;

pub enum Entry {
    Str(String),
    Int(i64),
    List(Vec<Entry>),
    Dictionary(HashMap<String, Entry>)
    
}

impl ToString for Entry {
    fn to_string(self: &Entry) -> String {
        match self {
            &Entry::Str(ref v) => v.to_string(),
            &Entry::Int(ref v) => v.to_string(),
            &Entry::List(ref v) => v.iter().fold("".to_string(), |l, v| l + &v.to_string()),
            &Entry::Dictionary(ref _v) => "TODO: Dict to string".to_string()
        }
    }
}

fn next(input: &str) -> Result<char, NoneError> {
    Ok(input.chars().next()?)
}

pub fn skip(input: &mut &str, s: usize) {
    *input = &input[s..];
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
    let val = decode_num(input, &|i| i != 'e')?;
    skip(input, 1);
    Ok(Entry::Int(val))
}

fn decode_list(input: &mut &str) -> Result<Entry, NoneError> {
    let mut r_list = Vec::new();

    loop {
        if next(*input)? == 'e' {
            break;
        }

        r_list.push(decode(input)?);
    }

    skip(input, 1);

    Ok(Entry::List(r_list))
}

fn decode_dict(input: &mut &str) -> Result<Entry, NoneError> {
    let mut r_map = HashMap::new();

    loop {
        
        if next(*input)? == 'e' {
            break;
        }

        let entry_name = (decode_str(input)?).to_string();
        let entry_val = decode(input)?;        

        r_map.insert(entry_name, entry_val);
    }

    skip(input, 1);

    Ok(Entry::Dictionary(r_map))
}

fn decode_str(input: &mut &str) -> Result<Entry, NoneError> {
    let str_len = decode_num(input, &|i| i != ':')? as usize;
    skip(input, 1);
    let res = &input[0..str_len];
    *input = &input[str_len..];
    Ok(Entry::Str(res.to_string()))
}

pub fn decode(input: &mut &str) -> Result<Entry, NoneError> {
    let id = next(input)?;
    match id {
        'i' => {
            skip(input, 1);
            decode_int(input)
        },
        'l' => {
            skip(input, 1);
            decode_list(input)
        },
        'd' => {
            skip(input, 1);
            decode_dict(input)
        },
        _ => decode_str(input)
    }
}

#[cfg(test)]
mod tests {
    use bencoder::decode;

    #[test]
    fn string() {
        let mut input = "5:doggy";
        assert_eq!(decode(&mut input).unwrap().to_string(), "doggy");
    }

    #[test]
    fn int() {
        let mut input = "i232e";
        assert_eq!(decode(&mut input).unwrap().to_string(), "232");
    }
}
