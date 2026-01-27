# Seeed

> [!NOTE]
> This project is currently a prototype / work-in-progress.

**Seeed** (with 3 'e's) is a simple yet powerful scripting language designed specifically to bootstrap and configure remote servers.

While tools like Ansible, Chef, or Puppet are powerful, they often require complex configuration files, strict directory structures, or heavy dependencies. **Seeed** aims to be a lighter alternative for simple provisioning tasks, offering a straightforward syntax to clear tasks quickly.

## Features

-   **Simple Syntax**: Easy to learn scripting language focused on remote execution.
-   **SSH Integration**: Built-in SSH client using the SSH Agent for authentication.
-   **Templating**: Jinja2-style templating (`{{ variable }}`) for dynamic configurations.
-   **Remote Blocks**: distinct syntax to define commands that execute on the remote server.
-   **Local & Remote Context**: Handle local variables and file uploads seamlessly.

## Installation

You can install `seeed` directly from the repository using `cargo`:

```bash
cargo install --git https://github.com/christophemaillot/seeed.git
```

## Usage

Run a seed script against a target server:

```bash
seeed [--target <user>@<host>[:<port>]] <SCRIPT_FILE>
```

### Options

| Option | Shorthand | Description | Default |
| :--- | :--- | :--- | :--- |
| `--target` | `-t` | The target host (e.g., `user@192.168.1.10:22`). Optional if defined in script. | - |
| `--sudo` | `-s` | Run the script with `sudo` privileges on the remote host. | `false` |
| `--shell` | `-e` | The shell to use on the remote host. | `/bin/bash` |
| `--env` | | Path to an environment file (`.env`) to load variables from. | - |
| `--debug` | `-d` | Print debug information during execution. | `false` |

### Example

```bash
seeed --target admin@myserver.com -s ./setup.seeed
```

## Language Reference

### variables

Define variables using the `let` keyword. Supported types are strings and arrays of strings.

```seeed
# String assignment
let username = "johnsnow"

# Defining the target host within the script
let target = "admin@myserver.com:2222"

# Array assignment
let packages = ["nginx", "git", "curl"]

# Heredoc (multi-line string)
let config = <<<EOF
server {
    listen 80;
    server_name example.com;
}
EOF>>>
```

> [!TIP]
> **Target Resolution**: The target host is resolved in the following order:
> 1. CLI argument (`--target` or `-t`)
> 2. `target` variable defined in the script
>
> If neither is provided before a remote command is executed, the script will fail.

### Remote Blocks

Commands enclosed in a remote block are executed on the target server. The strict syntax requires lines to start with `|` and the block to be delimited by `+`.

```seeed
+
| apt-get update
| apt-get install -y nginx
+
```

### Templating

You can use variables inside remote blocks or other strings using `{{ variable_name }}` syntax.

```seeed
let user = "deploy"

+
| mkdir -p /home/{{ user }}
| chown {{ user }}:{{ user }} /home/{{ user }}
+
```

### Control Flow

Iterate over arrays using `for` loops.

```seeed
let users = ["alice", "bob"]

for u in $users {
    +
    | echo "Creating user {{ u }}"
    | useradd -m {{ u }}
    +
}
```

### Built-in Functions

-   **`echo(arg1, arg2, ...)`**: Prints values to the local console.
-   **`upload(source, destination)`**: Uploads a string or file content to a specific path on the remote server.

```seeed
# Uploading a generated config file
let nginx_conf = "..."
upload($nginx_conf, "/etc/nginx/sites-available/default")

# Uploading a local file
upload("./local_config.conf", "/etc/myapp/config.conf")
```

## Limitations

-   **Authentication**: Currently, `seeed` **only** supports SSH Agent authentication. Ensure your specific key is added to your agent (`ssh-add ~/.ssh/id_rsa`) before running.
-   **Error Handling**: The project is in early stages; invalid syntax or network errors may cause generic failures.

## TODOs

Planned features for future releases:

-   [ ] Improved error handling and reporting.
-   [ ] Support for SSH key files and password authentication.
-   [ ] More variable types (Boolean, Objects).
-   [ ] Additional control structures (`if`/`else`).
-   [ ] `download()` built-in function.