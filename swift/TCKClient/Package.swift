// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "TCKClient",
    platforms: [
        .iOS(.v15),
        .macOS(.v12),
    ],
    products: [
        .library(
            name: "TCKClient",
            targets: ["TCKClient"]
        ),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "TCKClient",
            dependencies: [],
            path: "Sources/TCKClient"
        ),
        .testTarget(
            name: "TCKClientTests",
            dependencies: ["TCKClient"],
            path: "Tests"
        ),
    ]
)
