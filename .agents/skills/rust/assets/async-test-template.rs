#[tokio::test]
async fn returns_expected_result_for_valid_input() {
    // arrange
    let service = build_service();

    // act
    let result = service.execute("input").await;

    // assert
    assert!(result.is_ok());
}
