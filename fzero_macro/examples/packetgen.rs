use fzero_macro::fzero_define_grammar;

// https://github.com/landaire/lain/tree/master/examples

pub fn generate_u64_range<const L: u64, const U: u64>(rng: &mut impl rand::Rng) -> Vec<u8> {
    let u = rng.gen_range(L..U);
    u.to_le_bytes().to_vec()
}

pub fn random_bytes(rng: &mut impl rand::Rng) -> Vec<u8> {
    let len = rng.gen_range(0..4096);
    let mut data = vec![0; len + 8];
    data[0..8].copy_from_slice(&len.to_le_bytes());
    rng.fill_bytes(&mut data[8..]);
    data
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
    offset => generate!(generate_u64_range::<0, 4096>),
    length_and_data => generate!(random_bytes),
});

fn main() -> std::io::Result<()> {
    use std::io::Write;

    let mut rng = rand::thread_rng();
    let out = PacketGrammar::generate_new(None, &mut rng);

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&out)?;

    std::io::Result::Ok(())
}
