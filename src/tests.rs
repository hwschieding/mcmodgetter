use super::*;
use modrinth::*;

#[tokio::test]
async fn get_project_sodium() {
    let client = create_client().expect("Client should be created");
    let project = get_project(&client, "AANobbMI").await.expect("should exist");
    assert_eq!(project.get_title(), "Sodium");
    assert_eq!(project.get_id(), "AANobbMI")
}

#[tokio::test]
async fn get_list_of_projects() {
    let client = create_client().expect("Client should be created");
    let ids_vec = vec![String::from("P7dR8mSH"), String::from("AANobbMI"), String::from("9s6osm5g")];
    let project_vec = get_projects_from_list(&client, &ids_vec).await;
    assert_eq!(project_vec[0].as_ref().expect("Should exist").get_title(), "Fabric API");
    assert_eq!(project_vec[1].as_ref().expect("Should exist").get_title(), "Sodium")
}

#[tokio::test]
async fn get_version_sodium() {
    let client = create_client().expect("Client should be created");
    let query = VersionQuery::build_query(
        &String::from("1.21.8"), 
        &String::from("fabric")
    );
    let versions = get_version(&client, "AANobbMI", &query).await.expect("should exist");
    let v = &versions[0];
    assert_eq!(v.id(), "7pwil2dy");
    assert_eq!(v.name(), "Sodium 0.7.3 for Fabric 1.21.8");
    assert_eq!(v.version_number(), "mc1.21.8-0.7.3-fabric");
    assert_eq!(v.files()[0].filename(), "sodium-fabric-0.7.3+mc1.21.8.jar");
    assert!(v.files()[0].primary());
}

#[tokio::test]
async fn get_top_version_sodium() {
    let client = create_client().expect("Client should be created");
    let query = VersionQuery::build_query(
        &String::from("1.21.8"), 
        &String::from("fabric")
    );
    let v = get_top_version(&client, "AANobbMI", &query).await.expect("Should work");
    assert_eq!(v.id(), "7pwil2dy");
    assert_eq!(v.name(), "Sodium 0.7.3 for Fabric 1.21.8");
    assert_eq!(v.version_number(), "mc1.21.8-0.7.3-fabric");
    assert_eq!(v.files()[0].filename(), "sodium-fabric-0.7.3+mc1.21.8.jar");
    assert!(v.files()[0].primary());
}

#[tokio::test]
async fn get_top_version_capes_with_dependencies() {
    let client = create_client().expect("Client should be created");
    let query = VersionQuery::build_query(
        &String::from("1.21.8"), 
        &String::from("fabric")
    );
    let v = get_top_version(&client, "89Wsn8GD", &query).await.expect("Should work");
    assert_eq!(v.id(), "GRuX8d2G");
    assert_eq!(v.name(), "[Fabric 1.21.6-8] Capes 1.5.9");
    let _p_id1 = String::from("P7dR8mSH");
    let _p_id2 = String::from("Ha28R6CL");
    assert!(matches!(v.dependencies()[0].project_id(), Some(_p_id1)));
    assert!(matches!(v.dependencies()[1].project_id(), Some(_p_id2)));
}

#[tokio::test]
async fn get_primary_file_for_latest_sodium() {
    let client = create_client().expect("Client should be created");
    let query = VersionQuery::build_query(
        &String::from("1.21.9,1.21.10"), 
        &String::from("fabric")
    );
    let v = get_top_version(&client, "AANobbMI", &query).await.expect("should exist");
    let file_index = search_for_primary_file(&v.files()).expect("Should be Some");
    assert_eq!(file_index, 0);
    assert!(v.files()[0].primary());
}

#[test]
fn build_modrinth_query() {
    let query = VersionQuery::build_query(&String::from("1.21.9,1.21.10"), &String::from("fabric"));
    assert_eq!(query.mcvs(), "[\"1.21.9\",\"1.21.10\"]");
    assert_eq!(query.loader(), "[\"fabric\"]");
}

#[test]
fn build_modrinth_query_from_empty() {
    let query = VersionQuery::build_query(&String::from(""), &String::from(""));
    assert_eq!(query.mcvs(), "[\"\"]");
    assert_eq!(query.loader(), "[\"\"]");
}

#[test]
fn build_modrinth_query_from_random() {
    let query = VersionQuery::build_query(&String::from("sba,d,ugyu,,,,w,asd"), &String::from("asd,asd,f,a,,s,w,das,d,"));
    assert_eq!(query.mcvs(), "[\"sba\",\"d\",\"ugyu\",\"\",\"\",\"\",\"w\",\"asd\"]");
    assert_eq!(query.loader(), "[\"asd\",\"asd\",\"f\",\"a\",\"\",\"s\",\"w\",\"das\",\"d\",\"\"]");
}

#[test]
fn parse_line_from_mmg_file() {
    let curse_line = String::from("349239 -curse");
    let modrinth_line1 = String::from("SRlzjEBS -modrinth");
    let modrinth_line2 = String::from("SRlzjEBS");
    let curse_parse = file_parse::parse_input_line(&curse_line).expect("should be some");
    let modrinth_parse1 = file_parse::parse_input_line(&modrinth_line1).expect("should be some");
    let modrinth_parse2 = file_parse::parse_input_line(&modrinth_line2).expect("should be some");
    assert!(matches!(curse_parse, file_parse::IdType::Curseforge("349239")));
    assert!(matches!(modrinth_parse1, file_parse::IdType::Modrinth("SRlzjEBS")));
    assert!(matches!(modrinth_parse2, file_parse::IdType::Modrinth("SRlzjEBS")));

    let empty = String::new();
    let empty_parse = file_parse::parse_input_line(&empty).expect("should be some");
    assert!(matches!(empty_parse, file_parse::IdType::Modrinth("")));
}