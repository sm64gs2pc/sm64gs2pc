use sm64gs2pc::DecompData;

/// Helper for creating patches with minimal boilerplate
fn gs_to_patch(decomp_data: &DecompData, name: &str, code: &str) -> String {
    let code = code.parse::<sm64gs2pc::gameshark::Code>().unwrap();
    let patch = decomp_data.gs_code_to_patch(name, code).unwrap();
    println!("{}", patch);
    patch
}

/// Helper to run test cases with a decomp data
fn patch_convert_test_cases(decomp_data: &DecompData) {
    // Sources for tests:
    //   * https://www.ign.com/faqs/2004/shindou-super-mario-64-rumble-pak-vers-game-shark-codes-573979
    //   * https://www.gamegenie.com/cheats/gameshark/n64/super_mario_64.html

    assert_eq!(
        gs_to_patch(
            decomp_data,
            "Have 180 Stars",
            "8120770C FFFF
8120770E FFFF
81207710 FFFF
81207712 FFFF
81207714 FFFF
81207716 FFFF
81207718 FFFF
8120771A FFFF
8120771C FFFF
8120771E FFFF
81207720 FFFF
81207722 FFFF
81207724 FFFF",
        ),
        "--- a/src/game/gameshark.c
+++ b/src/game/gameshark.c
@@ -4,2 +4,17 @@
 void run_gameshark_cheats(void) {
+
+    /* Have 180 Stars */
+    /* 8120770C FFFF */ gSaveBuffer.files[0][0].courseStars[0] = (gSaveBuffer.files[0][0].courseStars[0] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[1] = (gSaveBuffer.files[0][0].courseStars[1] & 0xffffffffffffff00) | 0xff;
+    /* 8120770E FFFF */ gSaveBuffer.files[0][0].courseStars[2] = (gSaveBuffer.files[0][0].courseStars[2] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[3] = (gSaveBuffer.files[0][0].courseStars[3] & 0xffffffffffffff00) | 0xff;
+    /* 81207710 FFFF */ gSaveBuffer.files[0][0].courseStars[4] = (gSaveBuffer.files[0][0].courseStars[4] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[5] = (gSaveBuffer.files[0][0].courseStars[5] & 0xffffffffffffff00) | 0xff;
+    /* 81207712 FFFF */ gSaveBuffer.files[0][0].courseStars[6] = (gSaveBuffer.files[0][0].courseStars[6] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[7] = (gSaveBuffer.files[0][0].courseStars[7] & 0xffffffffffffff00) | 0xff;
+    /* 81207714 FFFF */ gSaveBuffer.files[0][0].courseStars[8] = (gSaveBuffer.files[0][0].courseStars[8] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[9] = (gSaveBuffer.files[0][0].courseStars[9] & 0xffffffffffffff00) | 0xff;
+    /* 81207716 FFFF */ gSaveBuffer.files[0][0].courseStars[10] = (gSaveBuffer.files[0][0].courseStars[10] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[11] = (gSaveBuffer.files[0][0].courseStars[11] & 0xffffffffffffff00) | 0xff;
+    /* 81207718 FFFF */ gSaveBuffer.files[0][0].courseStars[12] = (gSaveBuffer.files[0][0].courseStars[12] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[13] = (gSaveBuffer.files[0][0].courseStars[13] & 0xffffffffffffff00) | 0xff;
+    /* 8120771A FFFF */ gSaveBuffer.files[0][0].courseStars[14] = (gSaveBuffer.files[0][0].courseStars[14] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[15] = (gSaveBuffer.files[0][0].courseStars[15] & 0xffffffffffffff00) | 0xff;
+    /* 8120771C FFFF */ gSaveBuffer.files[0][0].courseStars[16] = (gSaveBuffer.files[0][0].courseStars[16] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[17] = (gSaveBuffer.files[0][0].courseStars[17] & 0xffffffffffffff00) | 0xff;
+    /* 8120771E FFFF */ gSaveBuffer.files[0][0].courseStars[18] = (gSaveBuffer.files[0][0].courseStars[18] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[19] = (gSaveBuffer.files[0][0].courseStars[19] & 0xffffffffffffff00) | 0xff;
+    /* 81207720 FFFF */ gSaveBuffer.files[0][0].courseStars[20] = (gSaveBuffer.files[0][0].courseStars[20] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[21] = (gSaveBuffer.files[0][0].courseStars[21] & 0xffffffffffffff00) | 0xff;
+    /* 81207722 FFFF */ gSaveBuffer.files[0][0].courseStars[22] = (gSaveBuffer.files[0][0].courseStars[22] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseStars[23] = (gSaveBuffer.files[0][0].courseStars[23] & 0xffffffffffffff00) | 0xff;
+    /* 81207724 FFFF */ gSaveBuffer.files[0][0].courseStars[24] = (gSaveBuffer.files[0][0].courseStars[24] & 0xffffffffffffff00) | 0xff; gSaveBuffer.files[0][0].courseCoinScores[0] = (gSaveBuffer.files[0][0].courseCoinScores[0] & 0xffffffffffffff00) | 0xff;
 ",
    );

    assert_eq!(
        gs_to_patch(
            decomp_data,
            "Moon Jump",
            "D033AFA1 0020
8133B1BC 4220
D033B1BD 0020
8133B17C 0300
D033B1BD 0020
8133B17E 0880",
        ),
        "--- a/src/game/gameshark.c
+++ b/src/game/gameshark.c
@@ -4,2 +4,10 @@
 void run_gameshark_cheats(void) {
+
+    /* Moon Jump */
+    /* D033AFA1 0020 */ if ((gControllers[0].buttonDown & 0xff) == 0x20)
+    /* 8133B1BC 4220 */ *(uint32_t *) &gMarioStates[0].vel[1] = (*(uint32_t *) &gMarioStates[0].vel[1] & 0xffffffff0000ffff) | 0x42200000;
+    /* D033B1BD 0020 */ if ((*(uint32_t *) &gMarioStates[0].vel[1] & 0xff0000) == 0x200000)
+    /* 8133B17C 0300 */ gMarioStates[0].action = (gMarioStates[0].action & 0xffffffff0000ffff) | 0x3000000;
+    /* D033B1BD 0020 */ if ((*(uint32_t *) &gMarioStates[0].vel[1] & 0xff0000) == 0x200000)
+    /* 8133B17E 0880 */ gMarioStates[0].action = (gMarioStates[0].action & 0xffffffffffff0000) | 0x880;
 ",
    );

    assert_eq!(
        gs_to_patch(
            decomp_data,
            "Always have Metal Cap",
            "8133B176 0015",
        ),
        "--- a/src/game/gameshark.c
+++ b/src/game/gameshark.c
@@ -4,2 +4,5 @@
 void run_gameshark_cheats(void) {
+
+    /* Always have Metal Cap */
+    /* 8133B176 0015 */ gMarioStates[0].flags = (gMarioStates[0].flags & 0xffffffffffff0000) | 0x15;
 ",
    );

    assert_eq!(
        gs_to_patch(
            decomp_data,
            "Limbo Mario",
            "8033B3BC 00C0",
        ),
        "--- a/src/game/gameshark.c
+++ b/src/game/gameshark.c
@@ -4,2 +4,5 @@
 void run_gameshark_cheats(void) {
+
+    /* Limbo Mario */
+    /* 8033B3BC 00C0 */ gBodyStates[0].torsoAngle[0] = (gBodyStates[0].torsoAngle[0] & 0xffffffffffff00ff) | 0xc000;
 ",
    );
}

/// Run tests on static decomp data
#[test]
fn patch_convert_static() {
    patch_convert_test_cases(&sm64gs2pc::DECOMP_DATA_STATIC)
}

/// Run tests on loaded decomp data
#[test]
#[cfg(feature = "loader")]
fn patch_convert_loader() {
    use std::path::Path;

    // Irix's C compiler doesn't like long paths, so clone in `/tmp` to be safe
    let repo = std::env::temp_dir();

    let decomp_data = DecompData::load(
        &Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/baserom.us.z64")),
        &repo,
    );

    // We can't just assert that the loaded version is equal to
    // `DECOMP_DATA_STATIC`, because the loading process isn't completely
    // deterministic (certain symbols are loaded at the same address and shadow
    // each other).
    //
    // Instead, run all the tests on the loaded version.

    patch_convert_test_cases(&decomp_data)
}
