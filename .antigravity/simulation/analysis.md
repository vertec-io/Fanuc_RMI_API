# Simulation & Kinematics Analysis

## Overview
The `sim` crate provides a software emulator of a FANUC robot. This is critical for development (testing without hardware) and for the "Digital Twin" aspect of the system.

## Kinematics (`kinematics.rs`)
The kinematics engine is rigorous and research-grade.

### Methodology
-   **Source**: Based on the paper *"Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot"* by Abbes & Poisson (Robotics 2024).
-   **DH Parameters**: Uses **Modified Denavit-Hartenberg (DHm)** convention, which is standard for industrial robotics but complex to implement correctly.
-   **Joint Coupling**: Correctly models the mechanical coupling between Joint 2 and Joint 3 found in FANUC robots (`theta3 = J2 + J3`).

### Algorithms
-   **Forward Kinematics (FK)**: Calculates the Tool Center Point (TCP) position/orientation from joint angles. Used for the 3D visualizer and verifying IK.
-   **Inverse Kinematics (IK)**: Calculates joint angles from a target TCP pose.
    -   **Hybrid Solver**:
        1.  **Full Geometric**: Attempt to solve analytically using the 7-step method from the paper. This provides mathematically perfect solutions when possible.
        2.  **Simplified Geometric**: A fallback solver for poses that are achievable but numerically difficult for the strict solver (e.g., near singularities).
    -   **Optimization**: When multiple solutions exist (e.g., "Elbow Up" vs "Elbow Down"), the solver selects the configuration closest to the current robot state, ensuring smooth motion.

## Simulator Server (`main.rs`)
-   **Protocol Emulation**: Listen on TCP port 18735 (standard RMI port) and mimics the RMI protocol text-based JSON format.
-   **State Tracking**: Maintains internal state (Position, Joints, Configuration).
-   **Response Timing**: Deliberately mimics the timing characteristics of a real controller to ensuring the driver's timing logic (e.g., 2ms delays) works correctly.
