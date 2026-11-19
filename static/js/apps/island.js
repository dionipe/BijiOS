/**
 * DokuHub Dynamic Island
 * macOS-style Dynamic Island Notification & Alert System
 * Replaces native window.alert and provides fluid pill animations
 */

(function () {
  'use strict';

  // 1. Audio Synthesizer for Apple-like gentle glass chime
  function playAppleChime(type = 'info') {
    try {
      const AudioCtx = window.AudioContext || window.webkitAudioContext;
      if (!AudioCtx) return;
      const ctx = new AudioCtx();
      if (ctx.state === 'suspended') {
        ctx.resume();
      }

      const now = ctx.currentTime;
      const osc1 = ctx.createOscillator();
      const osc2 = ctx.createOscillator();
      const gain = ctx.createGain();

      if (type === 'error') {
        // Lower two-tone error chime
        osc1.type = 'sine';
        osc1.frequency.setValueAtTime(320, now);
        osc1.frequency.exponentialRampToValueAtTime(220, now + 0.25);
        osc2.type = 'triangle';
        osc2.frequency.setValueAtTime(480, now);
        osc2.frequency.exponentialRampToValueAtTime(330, now + 0.25);
      } else if (type === 'success') {
        // High crisp double bell (Apple Pay / Success tone)
        osc1.type = 'sine';
        osc1.frequency.setValueAtTime(880, now);
        osc1.frequency.setValueAtTime(1320, now + 0.1);
        osc2.type = 'triangle';
        osc2.frequency.setValueAtTime(1760, now + 0.1);
      } else if (type === 'warn') {
        // Subtle alert marimba
        osc1.type = 'sine';
        osc1.frequency.setValueAtTime(587.33, now); // D5
        osc1.frequency.setValueAtTime(880, now + 0.08); // A5
      } else {
        // Elegant soft glass ping
        osc1.type = 'sine';
        osc1.frequency.setValueAtTime(1046.5, now); // C6
        osc1.frequency.exponentialRampToValueAtTime(1567.98, now + 0.08); // G6
      }

      gain.gain.setValueAtTime(0.001, now);
      gain.gain.exponentialRampToValueAtTime(0.18, now + 0.03);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.55);

      osc1.connect(gain);
      if (osc2.frequency.value > 0) osc2.connect(gain);
      gain.connect(ctx.destination);

      osc1.start(now);
      if (osc2.frequency.value > 0) osc2.start(now);
      osc1.stop(now + 0.6);
      if (osc2.frequency.value > 0) osc2.stop(now + 0.6);

      setTimeout(() => {
        try { ctx.close(); } catch (e) { }
      }, 700);
    } catch (e) {
      // Audio autoplay policy might silently block before first user interaction
    }
  }

  const DokuIsland = {
    el: null,
    dismissTimer: null,
    queue: [],
    isShowing: false,
    isHovered: false,

    init() {
      if (document.getElementById('mac-dynamic-island')) {
        this.el = document.getElementById('mac-dynamic-island');
      } else {
        this.el = document.createElement('div');
        this.el.id = 'mac-dynamic-island';
        this.el.className = 'dynamic-island idle';
        document.body.appendChild(this.el);
      }

      this.el.addEventListener('mouseenter', () => {
        this.isHovered = true;
        if (this.dismissTimer) clearTimeout(this.dismissTimer);
      });

      this.el.addEventListener('mouseleave', () => {
        this.isHovered = false;
        if (this.isShowing) {
          this.scheduleDismiss(2500);
        }
      });

      this.el.addEventListener('click', (e) => {
        if (e.target.closest('.island-btn-close') || e.target.closest('.island-action-btn')) {
          // let action handler do its job
        } else if (this.isShowing) {
          this.dismiss();
        }
      });

      this.renderIdle();
    },

    renderIdle() {
      if (!this.el) return;
      this.el.className = 'dynamic-island idle';
      this.el.innerHTML = `
        <div class="island-pill-idle">
          <span class="island-sensor-dot"></span>
          <span class="island-camera-lens"></span>
          <span class="island-mic-dot"></span>
        </div>
      `;
    },

    getLang() {
      if (window.DokuI18n && window.DokuI18n.lang) return window.DokuI18n.lang;
      try {
        return localStorage.getItem('dokuhub_lang') || 'en';
      } catch (e) {
        return 'en';
      }
    },

    isId() {
      return this.getLang() === 'id';
    },

    translateMsg(msg, isId) {
      if (!msg || typeof msg !== 'string') return msg;

      const ID_TO_EN = {
        'Dokumen berhasil disimpan!': 'Document saved successfully!',
        'Notifikasi Dynamic Island aktif sempurna!': 'Dynamic Island notification active!',
        'Semua rumus dan sel berhasil dihitung ulang': 'All formulas and cells recalculated successfully',
        'Pengaturan widget diperbarui': 'Widget preferences updated',
        'Widget desktop ditampilkan': 'Desktop widgets shown',
        'Widget desktop disembunyikan': 'Desktop widgets hidden',
        'Catatan sticky telah dikosongkan': 'Sticky note cleared',
        'Selamat Datang di BijiOS Desktop. Siap bekerja!': 'Welcome to BijiOS Desktop. Ready to work!',
        'Laporan Sistem berhasil disalin ke clipboard!': 'System report copied to clipboard!',
        'Kata sandi berhasil diperbarui': 'Password updated successfully',
        'Kata sandi berhasil diubah': 'Password changed successfully',
        'Profil berhasil diperbarui': 'Profile updated successfully',
        'Profil berhasil disimpan': 'Profile saved successfully',
        'Foto avatar berhasil diunggah': 'Avatar photo uploaded successfully',
        'Pengguna baru berhasil ditambahkan': 'New user added successfully',
        'Perubahan pengguna berhasil disimpan': 'User updated successfully',
        'Pengguna berhasil dihapus': 'User deleted successfully',
        'Tautan berhasil disalin ke clipboard': 'Link copied to clipboard',
        'Gagal menyimpan dokumen': 'Failed to save document',
        'Gagal menghapus berkas': 'Failed to delete file',
        'Gagal mengunggah berkas': 'Failed to upload file',
        'Folder berhasil dibuat': 'Folder created successfully'
      };

      const EN_TO_ID = {
        'Document saved successfully!': 'Dokumen berhasil disimpan!',
        'Dynamic Island notification active!': 'Notifikasi Dynamic Island aktif sempurna!',
        'All formulas and cells recalculated successfully': 'Semua rumus dan sel berhasil dihitung ulang',
        'Widget preferences updated': 'Pengaturan widget diperbarui',
        'Desktop widgets shown': 'Widget desktop ditampilkan',
        'Desktop widgets hidden': 'Widget desktop disembunyikan',
        'Sticky note cleared': 'Catatan sticky telah dikosongkan',
        'Welcome to BijiOS Desktop. Ready to work!': 'Selamat Datang di BijiOS Desktop. Siap bekerja!',
        'System report copied to clipboard!': 'Laporan Sistem berhasil disalin ke clipboard!',
        'Password updated successfully': 'Kata sandi berhasil diperbarui',
        'Password changed successfully': 'Kata sandi berhasil diubah',
        'Profile updated successfully': 'Profil berhasil diperbarui',
        'Profile saved successfully': 'Profil berhasil disimpan',
        'Avatar photo uploaded successfully': 'Foto avatar berhasil diunggah',
        'New user added successfully': 'Pengguna baru berhasil ditambahkan',
        'User updated successfully': 'Perubahan pengguna berhasil disimpan',
        'User deleted successfully': 'Pengguna berhasil dihapus',
        'Link copied to clipboard': 'Tautan berhasil disalin ke clipboard',
        'Failed to save document': 'Gagal menyimpan dokumen',
        'Failed to delete file': 'Gagal menghapus berkas',
        'Failed to upload file': 'Gagal mengunggah berkas',
        'Folder created successfully': 'Folder berhasil dibuat'
      };

      if (!isId) {
        if (ID_TO_EN[msg]) return ID_TO_EN[msg];
        // Dynamic regex translation patterns
        if (msg.includes('berhasil disimpan')) {
          return msg.replace('Spreadsheet "', 'Spreadsheet "').replace('" berhasil disimpan!', '" saved successfully!');
        }
        if (msg.startsWith('Sel ') && msg.includes('disalin ke clipboard')) {
          return msg.replace('Sel ', 'Cell ').replace(' disalin ke clipboard', ' copied to clipboard');
        }
        if (msg.startsWith('Lokasi cuaca diubah ke ')) {
          return msg.replace('Lokasi cuaca diubah ke ', 'Weather location changed to ');
        }
      } else {
        if (EN_TO_ID[msg]) return EN_TO_ID[msg];
        if (msg.includes('saved successfully!')) {
          return msg.replace(' saved successfully!', ' berhasil disimpan!');
        }
        if (msg.startsWith('Cell ') && msg.includes('copied to clipboard')) {
          return msg.replace('Cell ', 'Sel ').replace(' copied to clipboard', ' disalin ke clipboard');
        }
        if (msg.startsWith('Weather location changed to ')) {
          return msg.replace('Weather location changed to ', 'Lokasi cuaca diubah ke ');
        }
      }
      return msg;
    },

    /**
     * Show a Dynamic Island notification
     * @param {Object|string} options
     *   title: Main title
     *   message: Body text
     *   type: 'info' | 'success' | 'warn' | 'error'
     *   duration: ms to stay active (default 4500)
     *   icon: custom emoji or svg
     *   action: { label, onClick }
     */
    show(options) {
      if (!this.el) this.init();

      let opt = {};
      if (typeof options === 'string') {
        opt = { message: options, type: 'info' };
      } else {
        opt = Object.assign({}, options);
      }

      const isId = this.isId();

      // Default type inference
      const type = opt.type || 'info';
      const duration = opt.duration || 4500;
      let title = opt.title;
      let icon = opt.icon;

      // Localize standard default titles
      const isDefaultTitle = !title || [
        'Berhasil', 'Success', 'Peringatan', 'Warning', 'Perhatian', 'Caution',
        'Terjadi Kesalahan', 'An Error Occurred', 'Informasi', 'Information',
        'Pemberitahuan', 'Notice', 'BijiOS Sistem', 'BijiOS System', 'DokuHub Sistem', 'DokuHub System', 'Cuaca', 'Weather'
      ].includes(title);

      if (isDefaultTitle) {
        if (type === 'success') title = isId ? 'Berhasil' : 'Success';
        else if (type === 'error') title = isId ? 'Peringatan' : 'Warning';
        else if (type === 'warn') title = isId ? 'Perhatian' : 'Caution';
        else if (title === 'Cuaca' || title === 'Weather') title = isId ? 'Cuaca' : 'Weather';
        else title = isId ? 'BijiOS Sistem' : 'BijiOS System';
      }

      if (!icon) {
        if (type === 'success') icon = '✓';
        else if (type === 'error') icon = '✕';
        else if (type === 'warn') icon = '⚠️';
        else icon = '✦';
      }

      // Translate common message if needed
      const rawMessage = opt.message || '';
      const displayMessage = this.translateMsg(rawMessage, isId);

      // Play soft chime
      playAppleChime(type);

      // Render expanded state
      this.el.className = `dynamic-island expanded type-${type}`;
      this.isShowing = true;

      const hasAction = opt.action && opt.action.label;
      const actionHtml = hasAction ? `
        <button class="island-action-btn" id="island-action-btn">${opt.action.label}</button>
      ` : '';

      this.el.innerHTML = `
        <div class="island-content-wrap">
          <div class="island-leading-badge badge-${type}">
            <span class="island-icon">${icon}</span>
          </div>
          <div class="island-text-block">
            <div class="island-title-row">
              <span class="island-title">${title}</span>
              <span class="island-time">${isId ? 'sekarang' : 'now'}</span>
            </div>
            <div class="island-message">${displayMessage}</div>
          </div>
          <div class="island-trailing">
            ${actionHtml}
            <button class="island-btn-close" title="${isId ? 'Tutup' : 'Close'}">×</button>
          </div>
        </div>
        <div class="island-progress-bar">
          <div class="island-progress-fill" style="animation-duration: ${duration}ms;"></div>
        </div>
      `;

      if (hasAction && opt.action.onClick) {
        const btn = document.getElementById('island-action-btn');
        if (btn) {
          btn.onclick = (e) => {
            e.stopPropagation();
            try { opt.action.onClick(); } catch (err) { console.error(err); }
            this.dismiss();
          };
        }
      }

      const closeBtn = this.el.querySelector('.island-btn-close');
      if (closeBtn) {
        closeBtn.onclick = (e) => {
          e.stopPropagation();
          this.dismiss();
        };
      }

      this.scheduleDismiss(duration);
    },

    scheduleDismiss(ms) {
      if (this.dismissTimer) clearTimeout(this.dismissTimer);
      this.dismissTimer = setTimeout(() => {
        if (!this.isHovered) {
          this.dismiss();
        }
      }, ms);
    },

    dismiss() {
      if (!this.el || !this.isShowing) return;
      if (this.dismissTimer) clearTimeout(this.dismissTimer);

      this.el.classList.add('collapsing');
      setTimeout(() => {
        this.isShowing = false;
        this.renderIdle();
      }, 400);
    },

    // Shorthand helpers
    success(message, title) {
      const isId = this.isId();
      this.show({ type: 'success', title: title || (isId ? 'Berhasil' : 'Success'), message, duration: 4000 });
    },

    error(message, title) {
      const isId = this.isId();
      this.show({ type: 'error', title: title || (isId ? 'Terjadi Kesalahan' : 'An Error Occurred'), message, duration: 6000 });
    },

    info(message, title) {
      const isId = this.isId();
      this.show({ type: 'info', title: title || (isId ? 'Informasi' : 'Information'), message, duration: 4500 });
    },

    warn(message, title) {
      const isId = this.isId();
      this.show({ type: 'warn', title: title || (isId ? 'Pemberitahuan' : 'Notice'), message, duration: 5000 });
    }
  };

  // Monkey-patch window.alert to automatically use Dynamic Island
  const nativeAlert = window.alert;
  window.alert = function (msg) {
    const isId = (window.DokuIsland && typeof DokuIsland.isId === 'function')
      ? DokuIsland.isId()
      : ((window.DokuI18n && window.DokuI18n.lang === 'id') || localStorage.getItem('dokuhub_lang') === 'id');

    if (typeof msg === 'string') {
      const lower = msg.toLowerCase();
      let type = 'info';
      let title = isId ? 'BijiOS Notifikasi' : 'BijiOS Notification';
      if (lower.includes('berhasil') || lower.includes('success') || lower.includes('saved') || lower.includes('tersimpan')) {
        type = 'success';
        title = isId ? 'Berhasil' : 'Success';
      } else if (lower.includes('gagal') || lower.includes('error') || lower.includes('failed') || lower.includes('tidak dapat')) {
        type = 'error';
        title = isId ? 'Gagal' : 'Failed';
      } else if (lower.includes('peringatan') || lower.includes('warn') || lower.includes('caution') || lower.includes('penuh')) {
        type = 'warn';
        title = isId ? 'Peringatan' : 'Warning';
      }
      DokuIsland.show({ type, title, message: msg });
    } else {
      DokuIsland.show({ type: 'info', title: isId ? 'BijiOS Notifikasi' : 'BijiOS Notification', message: String(msg) });
    }
  };
  window.alert.__is_island_patched = true;
  window.alert.__native = nativeAlert;

  // Expose globally
  window.DokuIsland = DokuIsland;

  // Auto initialize on DOM ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => DokuIsland.init());
  } else {
    DokuIsland.init();
  }
})();
