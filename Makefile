PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
APP_ID ?= com.github.open_notch.OpenNotch

USER_BINDIR ?= $(HOME)/.local/bin
USER_DATADIR ?= $(HOME)/.local/share

.PHONY: all build install-user uninstall-user install uninstall

all: build
	@echo ""
	@echo "Build complete: target/release/open-notch"
	@echo "To install to your desktop app menu, run:"
	@echo "  make install-user   (installs for current user without sudo)"
	@echo "  sudo make install   (installs system-wide to /usr/local)"
	@echo ""

build:
	cargo build --release

install-user: build
	@echo "Installing OpenNotch for current user ($(USER))..."
	install -d "$(USER_BINDIR)"
	install -m 755 target/release/open-notch "$(USER_BINDIR)/open-notch"
	install -d "$(USER_DATADIR)/applications"
	sed 's|^Exec=open-notch|Exec=$(USER_BINDIR)/open-notch|' resources/open-notch.desktop > "$(USER_DATADIR)/applications/open-notch.desktop"
	chmod 644 "$(USER_DATADIR)/applications/open-notch.desktop"
	sed 's|^Exec=open-notch|Exec=$(USER_BINDIR)/open-notch|' resources/com.github.open_notch.OpenNotch.desktop > "$(USER_DATADIR)/applications/$(APP_ID).desktop"
	chmod 644 "$(USER_DATADIR)/applications/$(APP_ID).desktop"
	install -d "$(USER_DATADIR)/icons/hicolor"
	cp -r resources/icons/hicolor/* "$(USER_DATADIR)/icons/hicolor/"
	@command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$(USER_DATADIR)/applications" || true
	@command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$(USER_DATADIR)/icons/hicolor" || true
	@echo "Successfully installed to $(USER_BINDIR)/open-notch"
	@echo "Desktop entry: $(USER_DATADIR)/applications/open-notch.desktop"

uninstall-user:
	@echo "Uninstalling OpenNotch from current user..."
	rm -f "$(USER_BINDIR)/open-notch"
	rm -f "$(USER_DATADIR)/applications/open-notch.desktop"
	rm -f "$(USER_DATADIR)/applications/$(APP_ID).desktop"
	rm -f "$(USER_DATADIR)/icons/hicolor/scalable/apps/open-notch.svg"
	rm -f "$(USER_DATADIR)/icons/hicolor/scalable/apps/$(APP_ID).svg"
	rm -f "$(USER_DATADIR)/icons/hicolor/symbolic/apps/open-notch-symbolic.svg"
	rm -f "$(USER_DATADIR)/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg"
	@command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$(USER_DATADIR)/applications" || true
	@command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$(USER_DATADIR)/icons/hicolor" || true
	@echo "OpenNotch uninstalled."

install:
	install -d "$(DESTDIR)$(BINDIR)"
	install -m 755 target/release/open-notch "$(DESTDIR)$(BINDIR)/open-notch"
	install -d "$(DESTDIR)$(DATADIR)/applications"
	install -m 644 resources/open-notch.desktop "$(DESTDIR)$(DATADIR)/applications/open-notch.desktop"
	install -m 644 resources/com.github.open_notch.OpenNotch.desktop "$(DESTDIR)$(DATADIR)/applications/$(APP_ID).desktop"
	install -d "$(DESTDIR)$(DATADIR)/icons/hicolor"
	cp -r resources/icons/hicolor/* "$(DESTDIR)$(DATADIR)/icons/hicolor/"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/open-notch"
	rm -f "$(DESTDIR)$(DATADIR)/applications/open-notch.desktop"
	rm -f "$(DESTDIR)$(DATADIR)/applications/$(APP_ID).desktop"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/open-notch.svg"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/$(APP_ID).svg"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/symbolic/apps/open-notch-symbolic.svg"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg"
