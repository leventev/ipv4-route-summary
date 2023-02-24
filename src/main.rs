use std::{
    fmt,
    fs::File,
    io::{self, BufRead},
    path::Path,
};

#[repr(transparent)]
#[derive(Clone, Copy)]
/// The CIDR notation is stored instead of the actual bitmask
struct IPv4Mask(usize);

fn parse_octets(s: &str) -> Option<u32> {
    let octets_str: Vec<&str> = s.split('.').collect();
    if octets_str.len() != 4 {
        return None;
    }

    // unfortunately it is not possible to return from the parent function
    // in a closure
    let mut invalid = false;
    let octets: Vec<u8> = octets_str
        .iter()
        .map(|x| {
            x.parse::<u8>().unwrap_or_else(|_| {
                invalid = true;
                0
            })
        })
        .rev()
        .collect();

    if !invalid {
        let val = u32::from_ne_bytes(octets.try_into().unwrap());
        Some(val)
    } else {
        None
    }
}

impl IPv4Mask {
    fn parse(s: &str) -> Option<IPv4Mask> {
        let netid_bits = if s.contains('.') {
            let mask = match parse_octets(s) {
                Some(x) => x,
                None => return None,
            };

            // get how many bits long the host id is
            let host_bits = {
                let mut counter = 0;
                let mut temp_mask = mask;
                while temp_mask & (1 << 0) == 0 {
                    counter += 1;
                    temp_mask >>= 1;
                }

                counter
            };

            // check if the mask is valid
            if u32::MAX << host_bits != mask {
                return None;
            }

            32 - host_bits
        } else {
            match s.parse() {
                Ok(x) => {
                    if x > 32 {
                        return None;
                    }
                    x
                }
                Err(e) => panic!("{}", e),
            }
        };
        Some(IPv4Mask(netid_bits))
    }

    fn netid_mask(&self) -> u32 {
        if self.0 == 0 {
            return 0;
        }
        if self.0 == 32 {
            return u32::MAX;
        }
        u32::MAX.checked_shl(32 - self.0 as u32).unwrap_or(u32::MAX)
    }
}

impl fmt::Display for IPv4Mask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct IPv4Address(u32);

impl IPv4Address {
    fn parse(s: &str) -> Option<IPv4Address> {
        let addr = match parse_octets(s) {
            Some(x) => x,
            None => return None,
        };

        Some(IPv4Address(addr))
    }
}

impl fmt::Display for IPv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let octet1 = self.0 >> 24 & 0xff;
        let octet2 = self.0 >> 16 & 0xff;
        let octet3 = self.0 >> 8 & 0xff;
        let octet4 = self.0 & 0xff;

        write!(f, "{}.{}.{}.{}", octet1, octet2, octet3, octet4)
    }
}

fn create_summary_route(pairs: Vec<(IPv4Address, IPv4Mask)>) -> (IPv4Address, IPv4Mask) {
    let mut common_network_part_bits = 0;
    'outer: loop {
        for pair in pairs.iter() {
            let bit = 1 << (31 - common_network_part_bits);
            let ip = pair.0;
            if ip.0 & bit != pairs[0].0 .0 & bit {
                break 'outer;
            }
        }
        common_network_part_bits += 1;
    }

    let common_mask = IPv4Mask(common_network_part_bits);
    let common_ip = IPv4Address(pairs[0].0 .0 & common_mask.netid_mask());

    (common_ip, common_mask)
}

fn main() {
    let path = Path::new("test.txt");
    let display = path.display();

    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => panic!("can't open file {}: {}", display, e),
    };

    let lines = io::BufReader::new(file).lines();
    let mut pairs: Vec<(IPv4Address, IPv4Mask)> = Vec::new();

    for line in lines {
        if let Ok(l) = line {
            let parts: Vec<&str> = l.split('/').collect();
            if parts.len() != 2 {
                panic!("invalid line");
            }

            let addr = IPv4Address::parse(parts[0].trim()).unwrap();
            let mask = IPv4Mask::parse(parts[1].trim()).unwrap();
            if addr.0 != addr.0 & mask.netid_mask() {
                panic!("invalid mask: ip: {} mask: {}", addr, mask);
            }

            println!("{}/{}", addr, mask);
            pairs.push((addr, mask));
        }
    }

    let summary = create_summary_route(pairs);
    println!("summary: {} {}", summary.0, summary.1);
}
