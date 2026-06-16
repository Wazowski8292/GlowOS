# GlowOS

---

## Index

1. [How to Install and Run](#how-to-install-and-run)
2. [Terminal Commands](#terminal-commands)
3. [Technical Details](#technical-details)
4. [Todo](#todo)
5. [Long term goals](#long-term-goals)
6. [Current State](#current-state)
7. [AI](#AI)
8. [License](#license)
9. [Contributing](#contributing)

---

## How to Install and Run

### Prerequisites

Before running GlowOS, ensure you have the following installed on your system:

- **Rust**: You will need the Nightly toolchain and the `llvm-tools-preview` component for kernel development.
- **QEMU**: The emulator used to run the OS.
-**Open Virtual Machine Firmware**: An extra program for QEMU to run properly the kernel.
- **Bash**: A Unix-like shell to execute the startup script. You can run it and build it manually with cargo run if not.

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

3. **Run the OS in QEMU:**
   Cargo run will automaticly execute the provided bash script to compile the kernel and launch it inside a QEMU virtual machine:
   ```bash
   make run
   ```

4. **Test the OS:**
   Cargo test will automaticly execute the test that are provided:
   ```bash
   make test
   ```

5. **Run the OS in a USB:**
   Cargo run will automaticly execute the provided bash script to compile the kernel and deploy it to the USB:
   ```bash
   chmod +x deploy_usb.sh
   sudo ./deploy_usb.sh
   ```
   > Remeber to disactivate secure boot in the BIOS

---

## Terminal Commands

> Every command must be prefixed with `$`. The space between `$` and the command is optional — both `$help` and `$ help` are valid. And the commands are not case sensetive

| Command | Description |
|---|---|
| `$help` | Displays the name of all commands and a short description of each. |
| `$echo` | Prints anything you pass in. Arguments are separated by a space when displayed. |
| `$clear` | Clears the screen. |
| `$set_color` | Changes the color of the text and/or background. |
| `$update_color` | Updates all text on screen to use the currently set color. |
| `$xhci_log` | Shows xHCI's logs. |
| `$xhci_log_cap_register` | Shows xHCI's log capability registers. |
| `$xhci_log_op_register` | Shows xHCI's log operational registers. |
| `$holy_c` | Shows holy C logo. |

---

## Technical Details

- **Booting**: The boot proces is done via UEFI.
- **Display**: Output is rendered via the **frame buffer**.
- **Memory Management**: The kernel implements **paging** for virtual memory and **dynamic memory allocation** for heap usage at runtime.
- **Interrupts**: Interrupt handling is set up to manage hardware and software events.
- **USB**: USB support via **xHCI** is currently in progress.
- **Testing**: The kernel includes **cargo tests** for verifying core functionality.

---

## Todo

- [x] Improve and make a somewhat finished README
- [x] Add queue to the vga to stop dead locks -> I just don't print
- [x] Add a font renderer
- [x] Add print functions
- [x] Add abstraction for the print functions
- [x] Scroll up and down → No clear line when chars reach it
- [ ] Update the function that handled changing the text and background color
- [ ] Add a history of commands and access it with arrow keys
- [ ] Reset xHCI controler
- [ ] Add a way to insert letters in the middle of words without erasing them


---

## Long term goals

- [ ] Read and write to USB
- [ ] Add a config file for visuals
- [ ] Add multithreading
- [ ] Merge linked list
- [ ] File system
- [ ] Add userspace
- [ ] Add a scheduler
- [ ] Delete all dependecies

---

## Current State

![Kernel Demo](assets/GlowOS_16-6-2026.GIF)

---

## AI

During the development of this project AI has been use only to help gather information about kernels, make simple bash scripts, or add simple functions that have been **verifide by me**. Every other single line of code in this repository has been **writen by me** or has been copy and pasted from some amazing blogs that i have found online.

---

## License

This project is licensed under the **MIT License**.
See the `LICENSE` file for more details.

---

## Contributing

Contributions, ideas, and optimizations are welcome!
Feel free to open issues or submit pull requests.