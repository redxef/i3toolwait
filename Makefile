EXEC := i3toolwait
INSTALL_BASE ?= /usr/local

default: target/debug/${EXEC}
release: target/release/${EXEC}
default: target/debug/${EXEC}

install: target/release/${EXEC}
	install -Dm0755 -oroot -groot $< ${INSTALL_BASE}/bin/${EXEC}

target/release/${EXEC}:
	@cargo build --release

target/debug/${EXEC}:
	@cargo build

.PHONY: install
