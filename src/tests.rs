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

    let conf = arguments::ConfigBuilder::new_from_args(&args).expect("Conf builder should build").build().expect("Conf should build");
    let settings = match conf.mode()
    {
        arguments::AppMode::Download(s) => s,
        _ => { assert!(false); return; }
    };
    assert_eq!(settings.version_search(), "26.3");
    assert_matches!(settings.platform().api(), arguments::Api::Modrinth);
    assert_matches!(settings.platform().get().get(0).expect("At least one platform should be present"), arguments::Platform::Fabric);
    assert_eq!(settings.platform().get().len(), 1);
}

#[test]
fn config2()
{
    let args: Vec<String> = 
    vec![
        "", // represents executable name (always fist arg)
        "download",
        "-file",
        "filename.txt",
        "-mcv",
        "1.20.1,1.20.2",
        "--skipdeps",
        "-modrinth",
        "forge,fabric"
    ].into_iter()
        .map(|elem| String::from(elem))
        .collect()
    ;

    let conf = arguments::ConfigBuilder::new_from_args(&args).expect("Conf builder should build").build().expect("Conf should build");
    let settings = match conf.mode()
    {
        arguments::AppMode::Download(s) => s,
        _ => { assert!(false); return; }
    };    

    assert_eq!(settings.version_search(), "1.20.1,1.20.2");
    assert_matches!(settings.platform().api(), arguments::Api::Modrinth);
    let platform_strings: Vec<_> = settings.platform().get().iter().map(|p| p.to_str()).collect();
    assert!(platform_strings.contains(&"fabric"));
    assert!(platform_strings.contains(&"forge"));
    assert_eq!(settings.platform().get().len(), 2);

}

#[test]
fn config3()
{
    let args: Vec<String> = vec![
        "", // represents executable name (always fist arg)
        "readmods",
        "-file",
        "filename.txt",
    ].into_iter()
        .map(|elem| String::from(elem))
        .collect()
    ;

    let conf = arguments::ConfigBuilder::new_from_args(&args).expect("Conf builder should build").build().expect("Conf should build");
    let settings = match conf.mode()
    {
        arguments::AppMode::ReadMods(s) => s,
        _ => { assert!(false); return; }
    };

    assert_matches!(settings.supplier(), arguments::IdSupplier::File(_))
}

#[test]
fn modrinth_query_building()
{
    let args: Vec<String> = 
    vec![
        "", // represents executable name (always fist arg)
        "download",
        "-file",
        "filename.txt",
        "-mcv",
        "1.20.1,1.20.2",
        "--skipdeps",
        "-modrinth",
        "forge,fabric"
    ].into_iter()
        .map(|elem| String::from(elem))
        .collect()
    ;

    let conf = arguments::ConfigBuilder::new_from_args(&args).expect("Conf builder should build").build().expect("Conf should build");
    let settings = match conf.mode()
    {
        arguments::AppMode::Download(s) => s,
        _ => { assert!(false); return; }
    };    

    let v_query = modrinth::VersionQuery::build_query(settings.version_search(), settings.platform().get());
    assert_eq!(v_query.loader(), "[\"forge\",\"fabric\"]");
    assert_eq!(v_query.mcvs(), "[\"1.20.1\",\"1.20.2\"]");
}