PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
MANDIR ?= $(PREFIX)/share/man/man1

.PHONY: man install uninstall

man:
	cargo run --bin gen-man --release -- --out-dir target/man

install: man
	cargo build --release
	install -d $(BINDIR) $(MANDIR)
	install -m 755 target/release/skillsync $(BINDIR)/skillsync
	install -m 644 target/man/*.1 $(MANDIR)/

uninstall:
	rm -f $(BINDIR)/skillsync
	rm -f $(MANDIR)/skillsync*.1
