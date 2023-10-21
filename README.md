# 23C2-mips_squad

Repo for Rust Taller De Programacion 1 FIUBA

# Test client

1. Iniciar servidor:

En `git`

```
mkdir server-repo
cd server-repo
git init
touch testfile
add testfile
git add testfile
git commit -m hi
cd .git
touch git-daemon-export-ok
```

`sh start-daemon.rs`

2. en `git`

`clear; cargo run --bin fetch`

# Bibliografía de referencia

https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols
https://git-scm.com/docs/pack-protocol#_git_transport
https://git-scm.com/docs/protocol-capabilities
https://git-scm.com/docs/protocol-common
https://www.git-scm.com/docs/git-daemon
https://git-scm.com/book/en/v2/Git-Internals-Packfiles
https://git-scm.com/docs/pack-format
