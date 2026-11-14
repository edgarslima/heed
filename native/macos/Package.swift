// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "RecorderBridge",
    platforms: [.macOS("14.2")],
    products: [.library(name: "RecorderBridge", type: .static, targets: ["RecorderBridge"])],
    targets: [
        .target(name: "RecorderBridge"),
        .testTarget(name: "RecorderBridgeTests", dependencies: ["RecorderBridge"]),
    ]
)
