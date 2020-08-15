use sm64gs2pc::DecompData;

fn main() {
    let decomp_data = DecompData::load();
    println!("{:#?}", decomp_data);
}
