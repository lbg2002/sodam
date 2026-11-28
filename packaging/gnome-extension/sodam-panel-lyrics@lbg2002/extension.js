import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const DEBOUNCE_MS = 30;
const WATCHDOG_MS = 15000;
const PROGRESS_MS = 1000;

function formatMs(ms) {
    const totalSec = Math.max(0, Math.floor((ms || 0) / 1000));
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

export default class SodaMPanelLyricsExtension extends Extension {
    enable() {
        const configDir = GLib.build_filenamev([GLib.get_user_config_dir(), 'sodam']);
        GLib.mkdir_with_parents(configDir, 0o755);
        this._statePath = GLib.build_filenamev([configDir, 'panel-lyrics-state']);
        this._socketPath = GLib.build_filenamev([
            GLib.get_user_runtime_dir() || GLib.get_tmp_dir(),
            'sodam-instance.sock',
        ]);

        try {
            this._ifaceSettings = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        } catch (_e) {
            this._ifaceSettings = null;
        }

        this._state = null;
        this._style = null;
        this._position = null;
        this._expanded = false;
        this._animating = false;
        this._savedDateLocation = null;
        this._lastRendered = {};

        this._debounceId = 0;
        this._watchdogId = 0;
        this._progressTimerId = 0;
        this._stageKeyId = 0;
        this._monitor = null;
        this._monitorSigId = 0;

        this._buildPanelButton();
        this._buildExpandedOverlay();

        // Pure event-driven file watch on ~/.config/sodam (zero polling)
        try {
            const dirFile = Gio.File.new_for_path(configDir);
            this._monitor = dirFile.monitor_directory(Gio.FileMonitorFlags.NONE, null);
            this._monitorSigId = this._monitor.connect('changed', (_mon, file, otherFile, _event) => {
                const name = file?.get_basename?.();
                const otherName = otherFile?.get_basename?.();
                if (name === 'panel-lyrics-state' || otherName === 'panel-lyrics-state')
                    this._scheduleRefresh();
            });
        } catch (_e) {
            this._monitor = null;
        }

        this._place('center-replace');
        this._refresh();
    }

    _buildPanelButton() {
        this._pillRoot = new St.BoxLayout({
            style_class: 'lumina-island-pill lumina-dark',
            y_align: Clutter.ActorAlign.CENTER,
        });

        this._smCover = new St.Widget({
            style_class: 'lumina-island-cover-sm',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._smFallbackIcon = new St.Icon({
            icon_name: 'audio-x-generic-symbolic',
            icon_size: 13,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._smCoverWrap = new St.Bin({
            y_align: Clutter.ActorAlign.CENTER,
            child: this._smFallbackIcon,
        });

        this._pillLabel = new St.Label({
            text: '',
            style_class: 'lumina-island-pill-label',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pillLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        this._statusDot = new St.Widget({
            style_class: 'lumina-island-status-dot',
            y_align: Clutter.ActorAlign.CENTER,
        });

        this._pillRoot.add_child(this._smCoverWrap);
        this._pillRoot.add_child(this._pillLabel);
        this._pillRoot.add_child(this._statusDot);

        this._pureLabel = new St.Label({
            text: '',
            style_class: 'lumina-lyrics-label',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pureLabel.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        this._contentBin = new St.Bin({
            y_align: Clutter.ActorAlign.CENTER,
            child: this._pillRoot,
        });

        this._button = new St.Button({
            style_class: 'lumina-panel-btn',
            reactive: true,
            can_focus: true,
            y_align: Clutter.ActorAlign.CENTER,
            child: this._contentBin,
        });

        this._button.connect('button-press-event', (_actor, event) => {
            const btn = event.get_button();
            if (btn === Clutter.BUTTON_MIDDLE) {
                this._sendCommand('toggle');
                return Clutter.EVENT_STOP;
            }
            if (btn === Clutter.BUTTON_PRIMARY) {
                if (this._style === 'lyrics')
                    this._sendCommand('show');
                else if (this._expanded)
                    this._collapse(false);
                else
                    this._expand();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });

        this._container = new St.Bin({
            visible: false,
            y_align: Clutter.ActorAlign.CENTER,
            child: this._button,
        });
    }

    _buildExpandedOverlay() {
        this._overlay = new St.Widget({
            layout_manager: new Clutter.FixedLayout(),
            reactive: true,
            visible: false,
        });

        this._scrim = new St.Widget({
            style_class: 'lumina-island-scrim',
            reactive: true,
        });
        this._scrim.connect('button-press-event', () => {
            this._collapse(false);
            return Clutter.EVENT_STOP;
        });

        this._islandCard = new St.BoxLayout({
            vertical: true,
            reactive: true,
            style_class: 'lumina-island-card lumina-dark',
        });
        // Consume clicks inside the card so scrim doesn't collapse on button gaps
        this._islandCard.connect('button-press-event', () => Clutter.EVENT_STOP);

        // Header: Artwork + Title/Artist/Provider + Open Window
        const header = new St.BoxLayout({
            style_class: 'lumina-island-header',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._lgCover = new St.Widget({
            style_class: 'lumina-island-art-lg',
            y_align: Clutter.ActorAlign.CENTER,
        });
        const meta = new St.BoxLayout({
            vertical: true,
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
            style_class: 'lumina-island-meta',
        });
        this._cardTitle = new St.Label({
            text: 'Lumina',
            style_class: 'lumina-island-title',
        });
        this._cardTitle.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        this._cardArtist = new St.Label({
            text: '',
            style_class: 'lumina-island-artist',
        });
        this._cardArtist.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        this._cardSource = new St.Label({
            text: '汽水音乐',
            style_class: 'lumina-island-source',
            x_align: Clutter.ActorAlign.START,
        });

        const openIcon = new St.Icon({icon_name: 'window-new-symbolic', icon_size: 14});
        const openBtn = new St.Button({
            style_class: 'lumina-island-open-btn',
            child: openIcon,
            reactive: true,
            can_focus: true,
            y_align: Clutter.ActorAlign.START,
        });
        openBtn.connect('clicked', () => {
            this._sendCommand('show');
            this._collapse(false);
        });

        meta.add_child(this._cardTitle);
        meta.add_child(this._cardArtist);
        meta.add_child(this._cardSource);
        header.add_child(this._lgCover);
        header.add_child(meta);
        header.add_child(openBtn);

        // Lyrics box
        const lyricBox = new St.BoxLayout({
            vertical: true,
            style_class: 'lumina-island-lyric-box',
        });
        this._cardCurLyric = new St.Label({
            text: '',
            style_class: 'lumina-island-lyric-cur',
        });
        this._cardCurLyric.clutter_text.ellipsize = Pango.EllipsizeMode.END;

        this._cardNextLyric = new St.Label({
            text: '',
            style_class: 'lumina-island-lyric-next',
        });
        this._cardNextLyric.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        lyricBox.add_child(this._cardCurLyric);
        lyricBox.add_child(this._cardNextLyric);

        // Progress bar + mm:ss row (only ticked while expanded & playing)
        const progressWrap = new St.BoxLayout({
            vertical: true,
            style_class: 'lumina-island-progress-wrap',
        });
        this._progressTrack = new St.Widget({
            style_class: 'lumina-island-progress-bg',
            x_expand: true,
        });
        this._progressFill = new St.Widget({
            style_class: 'lumina-island-progress-fg',
        });
        this._progressFill.set_width(0);
        this._progressTrack.add_child(this._progressFill);

        const timeRow = new St.BoxLayout({
            style_class: 'lumina-island-time-row',
        });
        this._timeCurLabel = new St.Label({text: '00:00', x_expand: true});
        this._timeDurLabel = new St.Label({text: '00:00'});
        timeRow.add_child(this._timeCurLabel);
        timeRow.add_child(this._timeDurLabel);

        progressWrap.add_child(this._progressTrack);
        progressWrap.add_child(timeRow);

        // Transport controls
        const controls = new St.BoxLayout({
            style_class: 'lumina-island-controls',
            x_align: Clutter.ActorAlign.CENTER,
        });
        const makeCtrlBtn = (iconName, cmd, extraClass, iconSize = 16) => {
            const icon = new St.Icon({icon_name: iconName, icon_size: iconSize});
            const btn = new St.Button({
                style_class: extraClass,
                child: icon,
                reactive: true,
                can_focus: true,
            });
            btn.connect('clicked', () => this._sendCommand(cmd));
            return [btn, icon];
        };

        const [prevBtn] = makeCtrlBtn('media-skip-backward-symbolic', 'prev', 'lumina-island-ctrl-btn');
        const [playBtn, playIcon] = makeCtrlBtn(
            'media-playback-start-symbolic',
            'toggle',
            'lumina-island-ctrl-primary',
            18,
        );
        const [nextBtn] = makeCtrlBtn('media-skip-forward-symbolic', 'next', 'lumina-island-ctrl-btn');

        this._playBtn = playBtn;
        this._playIcon = playIcon;

        controls.add_child(prevBtn);
        controls.add_child(playBtn);
        controls.add_child(nextBtn);

        this._islandCard.add_child(header);
        this._islandCard.add_child(lyricBox);
        this._islandCard.add_child(progressWrap);
        this._islandCard.add_child(controls);

        this._overlay.add_child(this._scrim);
        this._overlay.add_child(this._islandCard);
        Main.layoutManager.addChrome(this._overlay, {
            affectsStruts: false,
            trackFullscreen: true,
        });
    }

    _scheduleRefresh() {
        if (this._debounceId)
            GLib.source_remove(this._debounceId);
        this._debounceId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, DEBOUNCE_MS, () => {
            this._debounceId = 0;
            this._refresh();
            return GLib.SOURCE_REMOVE;
        });
    }

    _syncWatchdog(shouldRun) {
        if (!shouldRun) {
            if (this._watchdogId) {
                GLib.source_remove(this._watchdogId);
                this._watchdogId = 0;
            }
            return;
        }
        if (this._watchdogId)
            return;
        this._watchdogId = GLib.timeout_add(GLib.PRIORITY_LOW, WATCHDOG_MS, () => {
            const pid = this._state?.pid || 0;
            if (pid > 0 && !GLib.file_test(`/proc/${pid}`, GLib.FileTest.EXISTS)) {
                this._watchdogId = 0;
                this._refresh();
                return GLib.SOURCE_REMOVE;
            }
            return GLib.SOURCE_CONTINUE;
        });
    }

    _syncClockPlacement(shouldReplaceClock) {
        const dateContainer = Main.panel.statusArea.dateMenu?.container ?? null;
        if (!dateContainer)
            return;

        if (shouldReplaceClock) {
            if (!this._savedDateLocation) {
                const parent = dateContainer.get_parent();
                if (parent) {
                    const index = Math.max(0, parent.get_children().indexOf(dateContainer));
                    this._savedDateLocation = {parent, index};
                }
            }
            const rightBox = Main.panel._rightBox;
            if (rightBox && dateContainer.get_parent() !== rightBox) {
                dateContainer.get_parent()?.remove_child(dateContainer);
                const quickContainer = Main.panel.statusArea.quickSettings?.container ?? null;
                if (quickContainer && quickContainer.get_parent() === rightBox) {
                    const idx = rightBox.get_children().indexOf(quickContainer);
                    rightBox.insert_child_at_index(dateContainer, Math.max(0, idx));
                } else {
                    rightBox.add_child(dateContainer);
                }
            }
        } else if (this._savedDateLocation) {
            const {parent, index} = this._savedDateLocation;
            this._savedDateLocation = null;
            if (parent && dateContainer.get_parent() !== parent) {
                dateContainer.get_parent()?.remove_child(dateContainer);
                parent.insert_child_at_index(
                    dateContainer,
                    Math.min(index, parent.get_n_children()),
                );
            }
        }
    }

    _place(position) {
        const container = this._container;
        if (!container)
            return;

        const normalized = ['center-replace', 'left', 'center-left', 'center-right', 'right'].includes(position)
            ? position
            : 'center-replace';

        const box = normalized === 'left'
            ? Main.panel._leftBox
            : normalized === 'right'
                ? Main.panel._rightBox
                : Main.panel._centerBox;
        if (!box)
            return;

        if (container.get_parent() !== box) {
            container.get_parent()?.remove_child(container);
            const idx = (normalized === 'right' || normalized === 'center-left' || normalized === 'center-replace')
                ? 0
                : box.get_n_children();
            box.insert_child_at_index(container, Math.min(idx, box.get_n_children()));
        }

        const dateContainer = Main.panel.statusArea.dateMenu?.container ?? null;
        if ((normalized === 'center-left' || normalized === 'center-right') &&
            dateContainer && dateContainer.get_parent() === box) {
            const children = box.get_children();
            const myIdx = children.indexOf(container);
            const dateIdx = children.indexOf(dateContainer);
            if (normalized === 'center-left' && myIdx !== dateIdx - 1)
                box.set_child_below_sibling(container, dateContainer);
            else if (normalized === 'center-right' && myIdx !== dateIdx + 1)
                box.set_child_above_sibling(container, dateContainer);
        }

        this._position = normalized;
    }

    _resolveIsDark(state) {
        if (typeof state?.isDark === 'boolean')
            return state.isDark;
        try {
            return this._ifaceSettings?.get_string('color-scheme') === 'prefer-dark';
        } catch (_e) {
            return true;
        }
    }

    _applyDarkLightClass(actor, isDark) {
        if (!actor)
            return;
        if (isDark) {
            actor.remove_style_class_name('lumina-light');
            actor.add_style_class_name('lumina-dark');
        } else {
            actor.remove_style_class_name('lumina-dark');
            actor.add_style_class_name('lumina-light');
        }
    }

    _readState() {
        try {
            const [ok, bytes] = GLib.file_get_contents(this._statePath);
            if (!ok)
                return null;
            const text = new TextDecoder().decode(bytes).trim();
            if (!text)
                return null;

            if (text.startsWith('{')) {
                const parsed = JSON.parse(text);
                if (!parsed || typeof parsed !== 'object')
                    return null;
                const pid = Number(parsed.pid) || 0;
                if (pid > 0 && !GLib.file_test(`/proc/${pid}`, GLib.FileTest.EXISTS))
                    return null;
                const accentRgb = Array.isArray(parsed.accent_rgb) && parsed.accent_rgb.length >= 3
                    ? parsed.accent_rgb.slice(0, 3).map(v => Math.max(0, Math.min(255, Number(v) || 0)))
                    : null;
                return {
                    enabled: Boolean(parsed.enabled),
                    style: parsed.style === 'lyrics' ? 'lyrics' : 'island',
                    position: parsed.position || 'center-replace',
                    lyric: (parsed.lyric || '').trim(),
                    nextLyric: (parsed.next_lyric || '').trim(),
                    color: /^#[0-9a-fA-F]{6}$/.test((parsed.color || '').trim())
                        ? parsed.color.trim()
                        : '#57D7E8',
                    isDark: typeof parsed.is_dark === 'boolean' ? parsed.is_dark : null,
                    accentRgb,
                    playing: Boolean(parsed.playing),
                    title: (parsed.title || '').trim(),
                    artist: (parsed.artist || '').trim(),
                    coverPath: (parsed.cover_path || '').trim(),
                    positionMs: Math.max(0, Number(parsed.position_ms) || 0),
                    durationMs: Math.max(0, Number(parsed.duration_ms) || 0),
                    updatedAtMs: Math.max(0, Number(parsed.updated_at_ms) || 0),
                    provider: parsed.provider === 'migu' ? 'migu' : 'soda',
                    pid,
                };
            }

            // Backward compatibility for legacy 4-line text state
            const lines = text.split(/\r?\n/);
            return {
                enabled: lines[0] === '1',
                style: 'lyrics',
                position: lines[1] || 'center-left',
                lyric: (lines[2] || '').trim(),
                nextLyric: '',
                color: /^#[0-9a-fA-F]{6}$/.test((lines[3] || '').trim())
                    ? lines[3].trim()
                    : '#FFFFFF',
                isDark: null,
                accentRgb: null,
                playing: true,
                title: '',
                artist: '',
                coverPath: '',
                positionMs: 0,
                durationMs: 0,
                updatedAtMs: 0,
                provider: 'soda',
                pid: 0,
            };
        } catch (_error) {
            return null;
        }
    }

    _applyColorToLabel(label, hex) {
        label.set_style(`color: ${hex};`);
        label.clutter_text.set_use_markup(false);
        const [ok, coglColor] = Cogl.Color.from_string(hex);
        if (ok)
            label.clutter_text.set_color(coglColor);
    }

    _updateCover(coverPath) {
        let coverUri = null;
        if (coverPath && GLib.file_test(coverPath, GLib.FileTest.EXISTS)) {
            try {
                coverUri = Gio.File.new_for_path(coverPath).get_uri();
            } catch (_e) {
                coverUri = null;
            }
        }
        if (coverUri === this._lastRendered.coverUri)
            return;
        this._lastRendered.coverUri = coverUri;
        if (coverUri) {
            const bgCss = `background-image: url("${coverUri}");`;
            this._smCover.set_style(bgCss);
            this._lgCover.set_style(bgCss);
            this._smCoverWrap.set_child(this._smCover);
        } else {
            this._smCover.set_style('');
            this._lgCover.set_style('');
            this._smCoverWrap.set_child(this._smFallbackIcon);
        }
    }

    _currentPositionMs(state) {
        if (!state)
            return 0;
        let pos = state.positionMs || 0;
        if (state.playing && state.updatedAtMs > 0)
            pos += Math.max(0, Date.now() - state.updatedAtMs);
        if (state.durationMs > 0)
            pos = Math.min(pos, state.durationMs);
        return Math.max(0, pos);
    }

    _updateProgressUi() {
        const state = this._state;
        const dur = state?.durationMs || 0;
        const pos = this._currentPositionMs(state);
        const frac = dur > 0 ? Math.max(0, Math.min(1, pos / dur)) : 0;
        const trackW = this._progressTrack.get_width();
        const totalW = trackW > 20 ? trackW : 336;
        this._progressFill.set_width(Math.round(totalW * frac));
        this._timeCurLabel.clutter_text.set_text(formatMs(pos));
        this._timeDurLabel.clutter_text.set_text(formatMs(dur));
    }

    _syncProgressTimer() {
        if (this._progressTimerId) {
            GLib.source_remove(this._progressTimerId);
            this._progressTimerId = 0;
        }
        if (!this._expanded || !this._state?.playing || !(this._state?.durationMs > 0))
            return;
        this._progressTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, PROGRESS_MS, () => {
            if (!this._expanded || !this._state?.playing) {
                this._progressTimerId = 0;
                return GLib.SOURCE_REMOVE;
            }
            this._updateProgressUi();
            return GLib.SOURCE_CONTINUE;
        });
    }

    _refresh() {
        const state = this._readState();
        this._state = state;

        const hasContent = Boolean(state && state.enabled && (
            state.style === 'lyrics' ? state.lyric : (state.lyric || state.title)
        ));
        if (!hasContent) {
            this._syncWatchdog(false);
            if (this._expanded)
                this._collapse(true);
            if (this._container)
                this._container.visible = false;
            this._syncClockPlacement(false);
            return;
        }

        const shouldReplaceClock = state.style === 'island' && state.position === 'center-replace';
        if (shouldReplaceClock)
            this._syncClockPlacement(true);
        if (this._position !== state.position)
            this._place(state.position);
        if (!shouldReplaceClock)
            this._syncClockPlacement(false);

        this._syncWatchdog(state.pid > 0);

        const isDark = this._resolveIsDark(state);
        const last = this._lastRendered;

        if (state.style === 'lyrics') {
            if (this._expanded)
                this._collapse(true);
            if (this._style !== 'lyrics') {
                this._style = 'lyrics';
                this._contentBin.set_child(this._pureLabel);
            }
            if (last.pureLyric !== state.lyric) {
                last.pureLyric = state.lyric;
                this._pureLabel.clutter_text.set_text(state.lyric);
            }
            if (last.pureColor !== state.color) {
                last.pureColor = state.color;
                this._applyColorToLabel(this._pureLabel, state.color);
            }
            this._container.visible = true;
            return;
        }

        if (this._style !== 'island') {
            this._style = 'island';
            this._contentBin.set_child(this._pillRoot);
        }

        const accentKey = state.accentRgb ? state.accentRgb.join(',') : state.color;
        if (last.isDark !== isDark || last.accentKey !== accentKey) {
            last.isDark = isDark;
            last.accentKey = accentKey;
            this._applyDarkLightClass(this._pillRoot, isDark);
            this._applyDarkLightClass(this._islandCard, isDark);
            if (this._expanded)
                this._attachBlur(isDark);
        }

        const pillText = state.lyric || state.title || 'Lumina';
        if (last.pillText !== pillText) {
            last.pillText = pillText;
            this._pillLabel.clutter_text.set_text(pillText);
        }
        if (last.color !== state.color) {
            last.color = state.color;
            this._applyColorToLabel(this._pillLabel, state.color);
            this._applyColorToLabel(this._cardCurLyric, state.color);
            this._playBtn.set_style(`background-color: ${state.color}; color: ${isDark ? '#0E1117' : '#FFFFFF'};`);
            this._progressFill.set_style(`background-color: ${state.color};`);
        }

        const dotStyle = state.playing
            ? `background-color: ${state.color}; opacity: 245;`
            : `background-color: ${isDark ? 'rgba(255,255,255,0.34)' : 'rgba(20,25,35,0.34)'};`;
        if (last.dotStyle !== dotStyle) {
            last.dotStyle = dotStyle;
            this._statusDot.set_style(dotStyle);
        }

        this._updateCover(state.coverPath);

        const titleText = state.title || 'Lumina';
        if (last.title !== titleText) {
            last.title = titleText;
            this._cardTitle.clutter_text.set_text(titleText);
        }
        const artistText = state.artist || '流光音乐';
        if (last.artist !== artistText) {
            last.artist = artistText;
            this._cardArtist.clutter_text.set_text(artistText);
        }
        const sourceText = state.provider === 'migu' ? '咪咕音乐' : '汽水音乐';
        if (last.provider !== sourceText) {
            last.provider = sourceText;
            this._cardSource.clutter_text.set_text(sourceText);
        }
        const curLyricText = state.lyric || titleText;
        if (last.curLyric !== curLyricText) {
            last.curLyric = curLyricText;
            this._cardCurLyric.clutter_text.set_text(curLyricText);
        }
        if (last.nextLyric !== state.nextLyric) {
            last.nextLyric = state.nextLyric;
            this._cardNextLyric.visible = Boolean(state.nextLyric);
            if (state.nextLyric)
                this._cardNextLyric.clutter_text.set_text(state.nextLyric);
        }
        if (last.playing !== state.playing) {
            last.playing = state.playing;
            this._playIcon.icon_name = state.playing
                ? 'media-playback-pause-symbolic'
                : 'media-playback-start-symbolic';
        }

        this._container.visible = true;
        if (this._expanded) {
            this._updateProgressUi();
            this._syncProgressTimer();
        }
    }

    _attachBlur(isDark) {
        try {
            if (this._islandCard.get_effect('lumina-blur'))
                this._islandCard.remove_effect_by_name('lumina-blur');
            const blur = new Shell.BlurEffect({
                mode: Shell.BlurMode.BACKGROUND,
                radius: 26,
                brightness: isDark ? 0.78 : 1.04,
            });
            this._islandCard.add_effect_with_name('lumina-blur', blur);
        } catch (_e) {
            // Pure CSS Liquid Glass fallback
        }
    }

    _detachBlur() {
        try {
            if (this._islandCard?.get_effect('lumina-blur'))
                this._islandCard.remove_effect_by_name('lumina-blur');
        } catch (_e) {
            // ignore
        }
    }

    _expand() {
        if (this._expanded || !this._state || this._style === 'lyrics')
            return;
        this._expanded = true;
        this._animating = true;

        const monitor = Main.layoutManager.primaryMonitor ?? Main.layoutManager.currentMonitor;
        this._overlay.set_position(monitor.x, monitor.y);
        this._overlay.set_size(monitor.width, monitor.height);
        this._scrim.set_position(0, 0);
        this._scrim.set_size(monitor.width, monitor.height);

        this._updateProgressUi();
        const isDark = this._resolveIsDark(this._state);
        this._attachBlur(isDark);

        const [bx] = this._button.get_transformed_position();
        const [bw] = this._button.get_transformed_size();
        const pillCenterX = bw > 0 ? bx + bw / 2 : monitor.x + monitor.width / 2;
        const cardW = this._islandCard.get_width() > 40 ? this._islandCard.get_width() : 368;
        const cardX = Math.round(Math.max(
            12,
            Math.min(monitor.width - cardW - 12, pillCenterX - monitor.x - cardW / 2),
        ));
        this._islandCard.set_position(cardX, 4);
        this._islandCard.set_pivot_point(0.5, 0.0);
        this._islandCard.set_scale(0.44, 0.26);
        this._islandCard.opacity = 0;

        this._overlay.visible = true;
        this._button.ease({
            opacity: 0,
            duration: 140,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });
        this._islandCard.ease({
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 255,
            duration: 220,
            mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
            onComplete: () => {
                this._animating = false;
            },
        });

        if (!this._stageKeyId) {
            this._stageKeyId = global.stage.connect('captured-event', (_stage, event) => {
                if (this._expanded && event.type() === Clutter.EventType.KEY_PRESS &&
                    event.get_key_symbol() === Clutter.KEY_Escape) {
                    this._collapse(false);
                    return Clutter.EVENT_STOP;
                }
                return Clutter.EVENT_PROPAGATE;
            });
        }
        this._syncProgressTimer();
    }

    _collapse(immediate = false) {
        this._expanded = false;
        if (this._progressTimerId) {
            GLib.source_remove(this._progressTimerId);
            this._progressTimerId = 0;
        }
        if (this._stageKeyId) {
            global.stage.disconnect(this._stageKeyId);
            this._stageKeyId = 0;
        }
        this._detachBlur();

        if (!this._overlay)
            return;

        if (immediate) {
            this._islandCard?.remove_all_transitions();
            this._button?.remove_all_transitions();
            this._overlay.visible = false;
            if (this._button)
                this._button.opacity = 255;
            this._animating = false;
            return;
        }

        this._animating = true;
        this._button?.ease({
            opacity: 255,
            duration: 150,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });
        this._islandCard?.ease({
            scale_x: 0.46,
            scale_y: 0.28,
            opacity: 0,
            duration: 150,
            mode: Clutter.AnimationMode.EASE_IN_CUBIC,
            onComplete: () => {
                if (!this._expanded && this._overlay)
                    this._overlay.visible = false;
                this._animating = false;
            },
        });
    }

    _sendCommand(cmd) {
        try {
            const addr = Gio.UnixSocketAddress.new(this._socketPath);
            const client = new Gio.SocketClient();
            client.connect_async(addr, null, (source, res) => {
                try {
                    const conn = source.connect_finish(res);
                    const out = conn.get_output_stream();
                    const payload = new TextEncoder().encode(`${cmd}\n`);
                    out.write_all(payload, null);
                    out.flush(null);
                    conn.close(null);
                } catch (_err) {
                    this._fallbackMpris(cmd);
                }
            });
        } catch (_err) {
            this._fallbackMpris(cmd);
        }
    }

    _fallbackMpris(cmd) {
        const map = {
            toggle: ['org.mpris.MediaPlayer2.Player', 'PlayPause'],
            play: ['org.mpris.MediaPlayer2.Player', 'Play'],
            pause: ['org.mpris.MediaPlayer2.Player', 'Pause'],
            next: ['org.mpris.MediaPlayer2.Player', 'Next'],
            prev: ['org.mpris.MediaPlayer2.Player', 'Previous'],
            show: ['org.mpris.MediaPlayer2', 'Raise'],
        };
        const target = map[cmd];
        if (!target)
            return;
        Gio.DBus.session.call(
            'org.mpris.MediaPlayer2.sodam',
            '/org/mpris/MediaPlayer2',
            target[0],
            target[1],
            null,
            null,
            Gio.DBusCallFlags.NO_AUTO_START,
            1500,
            null,
            null,
        );
    }

    disable() {
        if (this._debounceId) {
            GLib.source_remove(this._debounceId);
            this._debounceId = 0;
        }
        this._syncWatchdog(false);
        this._collapse(true);
        this._syncClockPlacement(false);

        if (this._monitor) {
            if (this._monitorSigId) {
                this._monitor.disconnect(this._monitorSigId);
                this._monitorSigId = 0;
            }
            this._monitor.cancel();
            this._monitor = null;
        }

        if (this._overlay) {
            Main.layoutManager.removeChrome(this._overlay);
            this._overlay.destroy();
            this._overlay = null;
        }
        if (this._container) {
            this._container.destroy();
            this._container = null;
        }

        this._button = null;
        this._pillRoot = null;
        this._pillLabel = null;
        this._pureLabel = null;
        this._contentBin = null;
        this._statusDot = null;
        this._islandCard = null;
        this._scrim = null;
        this._ifaceSettings = null;
        this._position = null;
        this._style = null;
        this._state = null;
        this._lastRendered = {};
    }
}
