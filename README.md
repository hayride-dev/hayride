# Hayride
Hayride is a Sandboxed execution environment powered by WebAssembly. It allows you to run untrusted code securely and efficiently in a controlled environment.

Hayride uses `wasmtime` as its WebAssembly runtime, and adds additional capabilities by implementing various WebAssembly Interfaces defined in [coven](https://github.com/hayride-dev/coven).

By using WIT (WebAssembly Interface Types), Hayride can seamlessly integrate with other WebAssembly components and services, allowing for greater flexibility and interoperability. Including implementing capabilities through WebAssembly Components vs host implementations and composing them into larger applications.

## Features

At its core, Hayride is simply a WebAssembly runtime that is designed to execute WebAssembly Components in a secure and isolated manner.
In addition to the core functionality, Hayride provides several features that enhance its usability and security.

### AI Inference: 
Hayride supports running AI models in a secure sandbox, enabling you to perform inference without exposing your system to potential threats. The AI feature allows you to deploy custom AI agents that can execute WebAssembly components as tools. 

## Building Hayride 

You can use the provided `Makefile` to build Hayride. The build process will compile the necessary components and prepare the environment for running WebAssembly applications.

### WebAssembly Component Dependencies

Hayride depends on WebAssembly Components that implement a number of Hayride interfaces. If you are building from source, you will need to get a copy of the `core` Hayride components.

Currently these are closed source but made freely available in our [releases](https://github.com/hayride-dev/releases) repository. These components are licensed under a "free" non-commercial use license. 

These components are required for Hayride to function properly, as they provide the necessary interfaces and capabilities that Hayride expects. However, these components are swappable with your own implementations, as long as they adhere to the expected interfaces.

We are working on improving the interfaces defined to ensure clarity on what is required to implement your own `core` components.

# Contributing
Contributions are welcome! If you'd like to contribute, please follow these steps:

- Fork the repository.
- Create a new branch for your feature or bug fix.
- Submit a pull request with a detailed description of your changes.

# License
This project is licensed under the AGPLv3 License. See the LICENSE file for details