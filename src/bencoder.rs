use std::collections::HashMap;
use std::option::NoneError;

#[derive(Debug)]
#[derive(Clone)]
pub enum Entry {
    Str(Vec<u8>),
    Int(i64),
    List(Vec<Entry>),
    Dictionary(HashMap<String, Entry>)    
}

impl ToString for Entry {
    fn to_string(self: &Entry) -> String {
        match self {
            &Entry::Str(ref v) => String::from_utf8_lossy(&v).to_string(),
            &Entry::Int(ref v) => v.to_string(),
            &Entry::List(ref v) => v.iter().fold("".to_string(), |l, v| l + ", "+ &v.to_string()),
            &Entry::Dictionary(ref _v) => "TODO: Dict to string".to_string()
        }
    }
}

impl Entry {
    pub fn field(self: &Entry, field: &str) -> Result<Entry, &'static str> {
        if let &Entry::Dictionary(ref d) = self {
            let info_portion = d.get(field);
            if info_portion.is_some() {
                Ok(info_portion.unwrap().clone())
            } else {
                Err("No Segment")
            }
        } else {
            Err("Not Dictionary")
        }
    }

    pub fn as_usize(self: &Entry) -> Result<usize, &'static str> {
        if let &Entry::Int(v) = self {
            Ok(v as usize)
        } else {
            Err("bad type")
        }
    }

    pub fn bencode(&self) -> Vec<u8> {
        match self {
            &Entry::Str(ref v) => (v.len().to_string() + ":").as_bytes().iter().chain(v).map(|x| *x).collect(),
            &Entry::Int(ref v) => ("i".to_string() + &v.to_string() + "e").as_bytes().iter().map(|x| *x).collect() 
        }
    }
}

fn next(input: &[u8]) -> Result<char, NoneError> {
    Ok(*input.iter().next()? as char)
}

pub fn skip(input: &mut &[u8], s: usize) {
    *input = &input[s..];
}

fn until<T: Fn(char) -> bool>(input: &mut &[u8], test: &T) -> Result<String, NoneError> {
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

fn decode_num<T: Fn(char) -> bool>(input: &mut &[u8], test: &T) -> Result<i64, NoneError> {
    let num_string = until(input, test)?;
    let r = num_string.parse::<i64>();
    
    match r {
        Ok(v) => Ok(v),
        Err(_e) => Err(NoneError)
    }
}

fn decode_int(input: &mut &[u8]) -> Result<Entry, NoneError> {
    let val = decode_num(input, &|i| i != 'e')?;
    skip(input, 1);
    Ok(Entry::Int(val))
}

fn decode_list(input: &mut &[u8]) -> Result<Entry, NoneError> {
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

fn decode_dict(input: &mut &[u8]) -> Result<Entry, NoneError> {
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

fn decode_str(input: &mut &[u8]) -> Result<Entry, NoneError> {
    let str_len = decode_num(input, &|i| i != ':')? as usize;
    skip(input, 1);
    let res = &input[0..str_len];
    *input = &input[str_len..];
    Ok(Entry::Str(res.to_vec()))
}

pub fn decode(input: &mut &[u8]) -> Result<Entry, NoneError> {
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

    #[test]
    fn list() {
        let mut input = "li232e5:doggye";
        assert_eq!(decode(&mut input).unwrap().to_string(), ", 232, doggy");
    }    
}
