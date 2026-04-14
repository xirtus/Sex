# Phase 12 Design: Dynamic Translators & URL-Based DSAS

## 🎯 Objective
Incorporate the structural advantages of **GNU Hurd** and **Redox OS** into the Sex Microkernel. This phase introduces **Dynamic Translators** (Hurd-style) and a **URL-driven Resource Scheme** (Redox-style) to make the Distributed Single Address Space (DSAS) more flexible, modular, and easy to navigate.

## 🏛 Architectural Vision: The "Plug-and-Play" OS

1.  **Sexting-Translators (Hurd-style):**
    *   In SexOS, a "Translator" is a Protection Domain (PD) that is attached to a specific `sexvfs` node.
    *   If a user accesses `/net/github.com`, the `sexvfs` dynamically spawns or routes the request to a `sexnet` translator PD.
    *   Unlike Hurd, our translators are **hardware-isolated via PKU**, making them both dynamic and stupidly fast.
2.  **Universal Resource URLs (Redox-style):**
    *   Everything in the DSAS is accessible via a URL scheme:
        *   `sexvfs://node1/etc/config`
        *   `sexnet://tcp/8080`
        *   `sexdrive://nvme0/partition1`
        *   `sexting://shared/buffer_alpha`
    *   This provides a unified mental model for developers: accessing a remote GPU or a local file uses the same syntax.

---

## 🗺 Implementation Roadmap

### 1. The Translator Engine (`sexvfs` expansion)
- [ ] **Node-PD Attachment:** Implement the ability to link a VFS path to a `Capability::IPC` of a specific translator PD.
- [ ] **On-Demand Translation:** If a node with an attached translator is accessed, `sexvfs` performs an implicit `safe_pdx_call` to the translator to resolve the request.

### 2. URL Resolver Server (`sexnode` expansion)
- [ ] **Scheme Registry:** Map schemes (e.g., `sexnet://`) to specific system servers.
- [ ] **Global URL Routing:** Use the distributed nature of `sexnode` to resolve URLs that point to remote machines (e.g., `sexdrm://workstation/display0`).

### 3. Benefit Incorporation
- **From Hurd:** We get **Extreme Modularity**. A user can write a custom filesystem translator in `sexc` and mount it anywhere without kernel permission.
- **From Redox:** We get **Architectural Cleanliness**. The kernel doesn't need to know what a "network socket" is; it just knows how to route a URL to a PD.

---

## 🧪 Phase 12 Verification
- **Dynamic Mount:** A user attaches a custom "Encrypted Folder" translator to `/home/user/vault`. Accessing that folder triggers the translator PD transparently.
- **URL Access:** A POSIX application opens `sexnet://google.com:80` using the standard `sexc::open()` call, and the system routes it to the `sexnet` PD.
- **Cross-Node URL:** A thread on the Pi 5 opens `sexdrive://x17r1/nvme0` and reads a block from the workstation's SSD via the distributed `sexvfs`.
