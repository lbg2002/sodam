import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import Pango from 'gi://Pango';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const REFRESH_MS = 200;
const LABEL_BASE_STYLE = 'max-width: 520px; padding-left: 8px; padding-right: 8px;';

export default class SodaMPanelLyricsExtension extends Extension {
    enable() {
        this._statePath = GLib.build_filenamev([
            GLib.get_user_config_dir(),
            'sodam',
            'panel-lyrics-state',
        ]);
        this._position = null;
        this._actor = new St.Bin({
            style_class: 'panel-button',
            reactive: false,
            can_focus: false,
            track_hover: false,
            visible: false,
        });
        this._label = new St.Label({
            text: '',
            y_align: Clutter.ActorAlign.CENTER,
            style: LABEL_BASE_STYLE,
        });
        this._label.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        this._actor.set_child(this._label);
        this._place('center-left');
        this._refresh();
        this._timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, REFRESH_MS, () => {
            this._refresh();
            return GLib.SOURCE_CONTINUE;
        });
    }

    _boxFor(position) {
        switch (position) {
        case 'left':
            return [Main.panel._leftBox, Main.panel._leftBox.get_n_children()];
        case 'center-right':
            return [Main.panel._centerBox, Main.panel._centerBox.get_n_children()];
        case 'right':
            return [Main.panel._rightBox, 0];
        case 'center-left':
        default:
            // Ubuntu 默认日期/时间位于 centerBox；插到索引 0 即位于日期时间左侧。
            return [Main.panel._centerBox, 0];
        }
    }

    _place(position) {
        if (!this._actor)
            return;
        const normalized = ['left', 'center-left', 'center-right', 'right'].includes(position)
            ? position
            : 'center-left';
        if (this._position === normalized && this._actor.get_parent())
            return;
        const parent = this._actor.get_parent();
        if (parent)
            parent.remove_child(this._actor);
        const [box, index] = this._boxFor(normalized);
        box.insert_child_at_index(this._actor, Math.min(index, box.get_n_children()));
        this._position = normalized;
    }

    _readState() {
        try {
            const [ok, bytes] = GLib.file_get_contents(this._statePath);
            if (!ok)
                return null;
            const text = new TextDecoder().decode(bytes);
            const lines = text.split(/\r?\n/);
            return {
                enabled: lines[0] === '1',
                position: lines[1] || 'center-left',
                lyric: (lines[2] || '').trim(),
                color: /^#[0-9a-fA-F]{6}$/.test((lines[3] || '').trim())
                    ? lines[3].trim()
                    : null,
            };
        } catch (_error) {
            return null;
        }
    }

    _refresh() {
        const state = this._readState();
        if (!state) {
            if (this._actor)
                this._actor.visible = false;
            return;
        }
        this._place(state.position);
        this._label.text = state.lyric;
        this._label.set_style(state.color
            ? `${LABEL_BASE_STYLE} color: ${state.color};`
            : LABEL_BASE_STYLE);
        this._actor.visible = state.enabled && state.lyric.length > 0;
    }

    disable() {
        if (this._timer) {
            GLib.source_remove(this._timer);
            this._timer = null;
        }
        if (this._actor) {
            this._actor.destroy();
            this._actor = null;
        }
        this._label = null;
        this._position = null;
    }
}
