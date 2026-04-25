# Makefile for brig-ssh
# Uses BSD make conventions. Run with `make` on FreeBSD.

DESTDIR?=
PREFIX?=	/usr/local
BINDIR=		${DESTDIR}${PREFIX}/bin
SHAREDIR=	${DESTDIR}${PREFIX}/share/brig

CARGO?=		cargo
CARGO_FLAGS=	--release

.PHONY: all build install clean test uninstall

all: build

build:
	${CARGO} build ${CARGO_FLAGS}

test:
	${CARGO} test

install: build
	install -m 0755 target/release/brig-ssh ${BINDIR}/brig-ssh
	install -d ${SHAREDIR}/skills/ssh-gateway
	install -m 0644 manifest.toml ${SHAREDIR}/skills/ssh-gateway/manifest.toml
	@echo ""
	@echo "Installed. To register with brig:"
	@echo "  brig skill add /usr/local/share/brig/skills/ssh-gateway"

clean:
	${CARGO} clean

uninstall:
	rm -f ${BINDIR}/brig-ssh
	rm -rf ${SHAREDIR}/skills/ssh-gateway
