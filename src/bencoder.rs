use std::collections::HashMap;
use std::option::NoneError;

/**
 * Entry Implementation, contains a decoded Bencode entry & its source string
 */

#[derive(Debug)]
#[derive(Clone)]
pub struct Entry {
    pub data: EntryData,
    pub src: Vec<u8>
}

impl Entry {

    pub fn from(data: EntryData, start_input: &[u8], end: usize) -> Entry { 
        Entry {
            data: data,
            src: start_input[0..start_input.len() - end].to_vec()
        }
    }

    pub fn field(&self, field: &str) -> Result<Entry, &'static str> {
        self.data.field(field)
    }

    pub fn as_usize(&self) -> Result<usize, &'static str> {
        self.data.as_usize()
    }
}

impl ToString for Entry {
    fn to_string(&self) -> String {
        self.data.to_string()
    }
}

/**
 * Enum represents the actual data of a bencoded entry
 */

#[derive(Debug)]
#[derive(Clone)]
pub enum EntryData {
    Str(Vec<u8>),
    Int(i64),
    List(Vec<Entry>),
    Dictionary(HashMap<String, Entry>)    
}

impl ToString for EntryData {
    fn to_string(&self) -> String {
        match self {
            &EntryData::Str(ref v) => String::from_utf8_lossy(&v).to_string(),
            &EntryData::Int(ref v) => v.to_string(),
            &EntryData::List(ref v) => v.iter().fold("".to_string(), |l, v| l + ", "+ &v.to_string()),
            &EntryData::Dictionary(ref _v) => "TODO: Dict to string".to_string()
        }
    }
}

impl EntryData {
    pub fn field(&self, field: &str) -> Result<Entry, &'static str> {
        if let &EntryData::Dictionary(ref d) = self {
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

    pub fn as_usize(&self) -> Result<usize, &'static str> {
        if let &EntryData::Int(v) = self {
            Ok(v as usize)
        } else {
            Err("bad type")
        }
    }
}

/**
 * What follows is the implementation of a simple bencoded parser
 */

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
    let start = input.clone(); 
    skip(input, 1);
    let val = decode_num(input, &|i| i != 'e')?;
    skip(input, 1);
    let end = input.len();
    Ok(Entry::from(EntryData::Int(val), start, end))
}

fn decode_list(input: &mut &[u8]) -> Result<Entry, NoneError> {
 
    let start = input.clone();

    skip(input, 1);

    let mut r_list = Vec::new();

    loop {
        if next(*input)? == 'e' {
            break;
        }

        r_list.push(decode(input)?);
    }

    skip(input, 1);

    let end = input.len();

    Ok(Entry::from(EntryData::List(r_list), start, end))
}

fn decode_dict(input: &mut &[u8]) -> Result<Entry, NoneError> {
    let start = input.clone();
 
    skip(input, 1);

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

    let end = input.len();

    Ok(Entry::from(EntryData::Dictionary(r_map), start, end))
}

fn decode_str(input: &mut &[u8]) -> Result<Entry, NoneError> {
    let start = input.clone();
    let str_len = decode_num(input, &|i| i != ':')? as usize;
    skip(input, 1);
    let res = &input[0..str_len];
    *input = &input[str_len..];

    let end = input.len();

    Ok(Entry::from(EntryData::Str(res.to_vec()), start, end))
}

pub fn decode(input: &mut &[u8]) -> Result<Entry, NoneError> {
    let id = next(input)?;
    match id {
        'i' => {
            decode_int(input)
        },
        'l' => {
            decode_list(input)
        },
        'd' => {
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
