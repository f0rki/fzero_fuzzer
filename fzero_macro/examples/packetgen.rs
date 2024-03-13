use fzero_macro::fzero_define_grammar;

// https://github.com/landaire/lain/tree/master/examples

pub fn generate_u64_range<const L: u64, const U: u64>(out: &mut Vec<u8>, rng: &mut impl rand::Rng) {
    let u = rng.gen_range(L..U);
    out.extend_from_slice(&u.to_le_bytes());
}

pub fn random_bytes(out: &mut Vec<u8>, rng: &mut impl rand::Rng) {
    let len: u64 = rng.gen_range(0..32);
    out.extend_from_slice(&len.to_le_bytes());
    let start = out.len();
    out.resize(start + (len as usize), 0);
    rng.fill_bytes(&mut out[start..]);
}

fzero_define_grammar!(PacketGrammar, [start], {
    start => [packets],
    packets => [packet, packets] | [packet],
    packet => [read_packet, write_packet, reset_packet],
    read_packet => [ b"\x00", packet_data ],
    write_packet => [ b"\x01", packet_data ],
    reset_packet => [ b"\x02", packet_data ],
    // packet_data => [offset, length, data],
    packet_data => [offset, length_and_data],
    offset => generate!(generate_u64_range::<0, 32>),
    length_and_data => generate!(random_bytes),
});

fn main() -> std::io::Result<()> {
    use std::io::Write;

    let mut rng = rand::thread_rng();
    let out = PacketGrammar::generate_new(Some(256), &mut rng);

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&out)?;

    std::io::Result::Ok(())
}
