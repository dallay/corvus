#[tokio::test]
async fn rejects_invalid_input_at_runtime_boundary() {
    // arrange runtime fixture

    // act
    let result = execute_boundary_call().await;

    // assert
    assert!(matches!(result, Err(_)));
}
