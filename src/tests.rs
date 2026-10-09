use std::assert_matches;

use super::*;

#[test]
fn config1()
{
    let args: Vec<String> = vec![
        "", // represents executable name (always fist arg)
        "download",
        "-id",
        "AABBCCDD",
        "-mcv",
        "26.3",
    ].into_iter()
        .map(|elem| String::from(elem))
        .collect()
    ;

    let conf = arguments::ConfigBuilder::new_from_args(&args).expect("Conf should build").build().expect("Conf should build");
    let settings = match conf.mode()
    {
        arguments::AppMode::Download(s) => s,
        _ => { assert!(false); return; }
    };
    assert_eq!(settings.version_search(), "26.3");
    assert_matches!(settings.platform(), arguments::UserPlatform::Modrinth(_));
}

// #[test]
// fn config2()
// {
//     let args: Vec<String> = 
//     vec![
//         "", // represents executable name (always fist arg)
//         "download",
//         "-file",
//         "filename.txt",
//         "-mcv",
//         "1.20.1",
//         "--skipdeps",
//         "-l",
//         "forge"
//     ].into_iter()
//         .map(|elem| String::from(elem))
//         .collect()
//     ;

//     let conf = arguments::RawConfig::build_from_args(&args).expect("Conf should build");
//     assert_matches!(conf.mode(), arguments::AppMode::DownloadFile);
//     assert_eq!(conf.mcvs(), "1.20.1");
//     assert_eq!(conf.loader().as_str(), "forge");
//     assert_eq!(conf.options().get_skip_deps(), true);
//     assert_eq!(conf.options().get_file().clone().expect("File should be present"), "filename.txt");
// }

// #[test]
// fn config3()
// {
//     let args: Vec<String> = vec![
//         "", // represents executable name (always fist arg)
//         "readmods",
//         "-file",
//         "filename.txt",
//     ].into_iter()
//         .map(|elem| String::from(elem))
//         .collect()
//     ;

//     let conf = arguments::RawConfig::build_from_args(&args).expect("Conf should build");
//     assert_matches!(conf.mode(), arguments::AppMode::ReadMods);
//     assert_eq!(conf.options().get_file().expect("File should be present"), "filename.txt");
// }