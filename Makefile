PREFIX    ?= /usr/local
BINDIR    ?= $(PREFIX)/bin
DATADIR   ?= $(PREFIX)/share
MANDIR    ?= $(DATADIR)/man/man1
COMPDIR   ?= $(DATADIR)/bash-completion/completions

CARGO     ?= cargo
CARGO_FLAGS ?= --release

.PHONY: all build install uninstall clean check

all: build

build:
	$(CARGO) build $(CARGO_FLAGS)

check:
	$(CARGO) test $(CARGO_FLAGS)

install: build
	install -Dm755 target/release/canvaswm   $(DESTDIR)$(BINDIR)/canvaswm
	install -Dm755 extras/canvaswm-msg        $(DESTDIR)$(BINDIR)/canvaswm-msg
	install -Dm644 example/canvaswm.toml      $(DESTDIR)$(DATADIR)/canvaswm/canvaswm.toml
	@echo "Installed canvaswm to $(DESTDIR)$(BINDIR)/canvaswm"

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/canvaswm
	rm -f $(DESTDIR)$(BINDIR)/canvaswm-msg
	rm -rf $(DESTDIR)$(DATADIR)/canvaswm

clean:
	$(CARGO) clean
