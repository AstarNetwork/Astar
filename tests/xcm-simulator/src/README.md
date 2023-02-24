# XCM Simulator Test Framework

The `xcm-simulator` framework provides a nice & easy way to test complex XCM behavior in the form of a unit test.

An entire system can be defined and/or mocked, which includes the relay chain and multiple parachains.
Sending of DMP, UMP and HRMP messages is supported.

Tester controls when XCM message is dispatched and when it's received by the destination chain.

# Structure

A custom relay-chain runtime is defined in this crate.

A custom parachain runtime is defined, loosely based on the `Shiden` runtime.
In current test setup, the same runtime is used to define two different parachains.
It's possible that in the future we decide to define a different runtime type (perhaps more resembling that of `Shibuya`).

# Running Tests

Running tests is same as with any other unit tests:

`cargo test -p xcm-simulator-tests`