use fanuc_rmi::ExtractInner;
use fanuc_rmi::commands::*;
use fanuc_rmi::instructions::*;
use fanuc_rmi::packets::*;
use fanuc_rmi::JointAngles;

#[test]
fn test_command_response_as_inner() {
    // Create a FrcReadJointAngles response
    let joint_angles = JointAngles {
        j1: 1.0,
        j2: 2.0,
        j3: 3.0,
        j4: 4.0,
        j5: 5.0,
        j6: 6.0,
        j7: 0.0,
        j8: 0.0,
        j9: 0.0,
    };

    let response = CommandResponse::FrcReadJointAngles(FrcReadJointAnglesResponse {
        error_id: 0,
        time_tag: 0,
        joint_angles: joint_angles.clone(),
        group: 1,
    });

    // Test as_inner with correct type
    let extracted: Option<&FrcReadJointAnglesResponse> = response.as_inner();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().error_id, 0);
    assert_eq!(extracted.unwrap().joint_angles.j1, 1.0);

    // Test as_inner with wrong type
    let wrong_type: Option<&FrcGetStatusResponse> = response.as_inner();
    assert!(wrong_type.is_none());
}

#[test]
fn test_command_response_into_inner() {
    // Create a FrcGetStatus response
    let response = CommandResponse::FrcGetStatus(FrcGetStatusResponse {
        error_id: 0,
        servo_ready: 1,
        tp_mode: 0,
        rmi_motion_status: 0,
        program_status: 0,
        single_step_mode: 0,
        number_utool: 1,
        number_uframe: 1,
    });

    // Test into_inner with correct type
    let extracted: Option<FrcGetStatusResponse> = response.into_inner();
    assert!(extracted.is_some());
    let status = extracted.unwrap();
    assert_eq!(status.error_id, 0);
    assert_eq!(status.servo_ready, 1);
}

#[test]
fn test_command_response_expect_inner() {
    // Create a FrcAbort response
    let response = CommandResponse::FrcAbort(FrcAbortResponse {
        error_id: 0,
    });

    // Test expect_inner with correct type
    let extracted: &FrcAbortResponse = response.expect_inner("Expected FrcAbort");
    assert_eq!(extracted.error_id, 0);
}

#[test]
#[should_panic(expected = "Expected FrcReadJointAngles")]
fn test_command_response_expect_inner_panic() {
    // Create a FrcAbort response
    let response = CommandResponse::FrcAbort(FrcAbortResponse {
        error_id: 0,
    });

    // This should panic because we're expecting the wrong type
    let _extracted: &FrcReadJointAnglesResponse = response.expect_inner("Expected FrcReadJointAngles");
}

#[test]
fn test_instruction_response_as_inner() {
    // Create a FrcLinearMotion response
    let response = InstructionResponse::FrcLinearMotion(FrcLinearMotionResponse {
        error_id: 0,
        sequence_id: 1,
    });

    // Test as_inner with correct type
    let extracted: Option<&FrcLinearMotionResponse> = response.as_inner();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().error_id, 0);
    assert_eq!(extracted.unwrap().sequence_id, 1);

    // Test as_inner with wrong type
    let wrong_type: Option<&FrcJointMotionResponse> = response.as_inner();
    assert!(wrong_type.is_none());
}

#[test]
fn test_instruction_response_into_inner() {
    // Create a FrcWaitTime response
    let response = InstructionResponse::FrcWaitTime(FrcWaitTimeResponse {
        error_id: 0,
        sequence_id: 42,
    });

    // Test into_inner with correct type
    let extracted: Option<FrcWaitTimeResponse> = response.into_inner();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().sequence_id, 42);
}

#[test]
fn test_communication_response_as_inner() {
    // Create a FrcConnect response
    let response = CommunicationResponse::FrcConnect(FrcConnectResponse {
        error_id: 0,
        port_number: 16001,
        major_version: 1,
        minor_version: 0,
    });

    // Test as_inner with correct type
    let extracted: Option<&FrcConnectResponse> = response.as_inner();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().port_number, 16001);

    // Test as_inner with wrong type
    let wrong_type: Option<&FrcDisconnectResponse> = response.as_inner();
    assert!(wrong_type.is_none());
}

#[test]
fn test_communication_response_into_inner() {
    // Create a FrcDisconnect response
    let response = CommunicationResponse::FrcDisconnect(FrcDisconnectResponse {
        error_id: 0,
    });

    // Test into_inner with correct type
    let extracted: Option<FrcDisconnectResponse> = response.into_inner();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap().error_id, 0);
}

#[test]
fn test_multiple_extractions() {
    // Test that we can extract different types from different responses
    let joint_angles = JointAngles {
        j1: 1.0, j2: 2.0, j3: 3.0, j4: 4.0, j5: 5.0, j6: 6.0, j7: 0.0, j8: 0.0, j9: 0.0,
    };

    let responses = vec![
        CommandResponse::FrcReadJointAngles(FrcReadJointAnglesResponse {
            error_id: 0,
            time_tag: 0,
            joint_angles: joint_angles.clone(),
            group: 1,
        }),
    ];

    for response in responses {
        let extracted: Option<&FrcReadJointAnglesResponse> = response.as_inner();
        if let Some(joint_angles_resp) = extracted {
            assert_eq!(joint_angles_resp.joint_angles.j1, 1.0);
        }
    }
}

