use sm64gs2pc::gameshark;
use sm64gs2pc::DecompData;

fn main() {
    let decomp_data = DecompData::load();
    let codes = "8129CE9C 2400\n\
                 8129CEC0 2400\n\
                 D033AFA1 0020\n\
                 8033B21E 0008\n\
                 D033AFA1 0020\n\
                 8133B262 0000\n\
                 D033AFA1 0020\n\
                 8133B218 0000\n\
                 D033AFA1 0020\n\
                 8033B248 0002\n\
                 D033AFA1 0020\n\
                 81361414 0005"
        .parse::<gameshark::Codes>()
        .unwrap();
    for code in codes.0 {
        let (_, decl) = decomp_data
            .decls
            .iter()
            .rev()
            .find(|(addr, _)| **addr <= code.addr() + 0x80000000)
            .unwrap();
        println!("name: {}", decl.name);
    }
    println!("{:#?}", decomp_data);
}
