use sm64gs2pc::gameshark;
use sm64gs2pc::DecompData;

fn main() {
    let decomp_data = DecompData::load();
    let codes = "D033AFA1 0020\n\
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
    let patch = decomp_data.gs_codes_to_patch(codes).unwrap();
    println!("{}", patch);
}
