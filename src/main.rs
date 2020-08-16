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
    for code in codes.0 {
        let addr = code.addr() + 0x80000000;
        let lvalue = decomp_data.addr_to_lvalue(addr).unwrap();

        match code {
            gameshark::Code::Write8 { value, .. } => {
                println!("{} = {:#x};", lvalue, value);
            }
            gameshark::Code::Write16 { value, .. } => {
                println!("{} = {:#x};", lvalue, value);
            }
            gameshark::Code::IfEq8 { value, .. } => {
                println!("if ({} == {:#x})", lvalue, value);
            }
            gameshark::Code::IfEq16 { value, .. } => {
                println!("if ({} == {:#x})", lvalue, value);
            }
            gameshark::Code::IfNotEq8 { value, .. } => {
                println!("if ({} != {:#x})", lvalue, value);
            }
            gameshark::Code::IfNotEq16 { value, .. } => {
                println!("if ({} != {:#x})", lvalue, value);
            }
        }
    }
    //println!("{:#?}", decomp_data);
}
