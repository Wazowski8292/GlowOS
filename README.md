# GlowOS

---

## Index

1. [How to Install and Run](#how-to-install-and-run)
2. [Terminal Commands](#terminal-commands)
3. [Technical Details](#technical-details)
4. [Todo](#todo)
5. [License](#license)
6. [Contributing](#contributing)

---

## How to Install and Run

### Prerequisites

Before running GlowOS, ensure you have the following installed on your system:

- **Rust**: You will need the Nightly toolchain and the `llvm-tools-preview` component for kernel development.
- **QEMU**: The emulator used to run the OS.
- **Bash**: A Unix-like shell to execute the startup script.

### Setup Instructions

1. **Clone the repository:**
   ```bash
   git clone https://github.com/Magicchess1244/GlowOS.git
   cd GlowOS
   ```

2. **Configure the Rust toolchain:**
   Make sure you are using the nightly channel and have the required components installed:
   ```bash
   rustup override set nightly
   rustup component add llvm-tools-preview
   ```

3. **Run the OS:**
   Cargo run will automaticly execute the provided bash script to compile the kernel and launch it inside a QEMU virtual machine:
   ```bash
   Cargo run
   ```

3. **Test the OS:**
   Cargo test will automaticly execute the test that are provided:
   ```bash
   Cargo test
   ```

---

## Terminal Commands

> Every command must be prefixed with `$`. The space between `$` and the command is optional — both `$help` and `$ help` are valid.

| Command | Description |
|---|---|
| `$help` | Displays the name of all commands and a short description of each. |
| `$echo` | Prints anything you pass in. Arguments are separated by a space when displayed. |
| `$clear` | Clears the screen. |
| `$set_color` | Changes the color of the text and/or background. |
| `$update_color` | Updates all text on screen to use the currently set color. |
| `$xhci_log_register` | Shows xHCI's log capability registers. |

---

## Technical Details

- **Display**: Output is rendered via the **VGA buffer**, writing directly to memory-mapped video memory.
- **Memory Management**: The kernel implements **paging** for virtual memory and **dynamic memory allocation** for heap usage at runtime.
- **Interrupts**: Interrupt handling is set up to manage hardware and software events.
- **USB**: USB support via **xHCI** is currently in progress.
- **Testing**: The kernel includes **cargo tests** for verifying core functionality.

---

## Todo

- [x] Improve and make a somewhat finished README
- [x] Scroll up and down → No clear line when chars reach it
- [x] Add queue to the vga to stop dead locks -> I just don't print
- [x] Add more commands to terminal
- [ ] Merge linked list
- [ ] Add a history of commands and access it with arrow keys
- [ ] Add a way to insert letters in the middle of words without erasing them
- [ ] File system
- [ ] Read and write to USB
- [ ] Add a config file for visuals
- [ ] Add userspace

---

## License

This project is licensed under the **MIT License**.
See the `LICENSE` file for more details.

---

## Contributing

Contributions, ideas, and optimizations are welcome!
Feel free to open issues or submit pull requests.