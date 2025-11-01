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
    let versions = get_version(&client, "AANobbMI").await.expect("should exist");
    let v = &versions[0];
    assert_eq!(v.id(), "h30oKQW3");
    assert_eq!(v.name(), "Sodium 0.7.2 for NeoForge 1.21.10");
    assert_eq!(v.version_number(), "mc1.21.10-0.7.2-neoforge");
    assert_eq!(v.files()[0].filename(), "sodium-neoforge-0.7.2+mc1.21.10.jar");
    assert!(v.files()[0].primary());
}

#[tokio::test]
async fn get_top_version_sodium() {
    let client = create_client().expect("Client should be created");
    let v = get_top_version(&client, "AANobbMI").await.expect("Should work");
    assert_eq!(v.id(), "h30oKQW3");
    assert_eq!(v.name(), "Sodium 0.7.2 for NeoForge 1.21.10");
    assert_eq!(v.version_number(), "mc1.21.10-0.7.2-neoforge");
    assert_eq!(v.files()[0].filename(), "sodium-neoforge-0.7.2+mc1.21.10.jar");
    assert!(v.files()[0].primary());
}

#[tokio::test]
async fn get_primary_file_for_latest_sodium() {
    let client = create_client().expect("Client should be created");
    let v = get_top_version(&client, "AANobbMI").await.expect("should exist");
    let file_index = search_for_primary_file(&v.files()).await.expect("Should be Some");
    assert_eq!(file_index, 0);
    assert!(v.files()[0].primary());
}