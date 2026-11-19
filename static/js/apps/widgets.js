/**
 * DokuHub macOS Desktop Widgets System
 * Authentic macOS Sonoma / Sequoia style interactive desktop widgets:
 * - Analog & Digital Clock with World Time
 * - Interactive Month Calendar with Event Agenda
 * - Dynamic Weather with Forecast & City Switcher
 * - System Resources (CPU, RAM, Personal Storage) Radial Gauges
 * - Quick Amber Glass Sticky Note with Auto-Save
 */

(function () {
  'use strict';

  const WEATHER_PRESETS = {
    'Jakarta': { temp: 29, high: 32, low: 24, cond: 'Cerah Berawan', cond_id: 'Cerah Berawan', cond_en: 'Partly Cloudy', icon: '⛅', aqi: 45, hourly: [
      { t: '12:00', icon: '☀️', temp: 31 },
      { t: '15:00', icon: '⛅', temp: 30 },
      { t: '18:00', icon: '🌦️', temp: 27 },
      { t: '21:00', icon: '🌙', temp: 26 }
    ]},
    'Bandung': { temp: 23, high: 26, low: 19, cond: 'Sejuk Berawan', cond_id: 'Sejuk Berawan', cond_en: 'Cool & Cloudy', icon: '🌤️', aqi: 28, hourly: [
      { t: '12:00', icon: '⛅', temp: 25 },
      { t: '15:00', icon: '🌧️', temp: 22 },
      { t: '18:00', icon: '🌧️', temp: 20 },
      { t: '21:00', icon: '🌙', temp: 19 }
    ]},
    'Surabaya': { temp: 32, high: 34, low: 26, cond: 'Cerah Terik', cond_id: 'Cerah Terik', cond_en: 'Sunny & Hot', icon: '☀️', aqi: 52, hourly: [
      { t: '12:00', icon: '☀️', temp: 34 },
      { t: '15:00', icon: '🌤️', temp: 33 },
      { t: '18:00', icon: '🌅', temp: 29 },
      { t: '21:00', icon: '🌙', temp: 27 }
    ]},
    'Bali': { temp: 30, high: 31, low: 25, cond: 'Cerah Tropis', cond_id: 'Cerah Tropis', cond_en: 'Tropical Sun', icon: '🌴', aqi: 22, hourly: [
      { t: '12:00', icon: '☀️', temp: 31 },
      { t: '15:00', icon: '🌤️', temp: 30 },
      { t: '18:00', icon: '🌅', temp: 28 },
      { t: '21:00', icon: '🌙', temp: 26 }
    ]},
    'Singapore': { temp: 28, high: 31, low: 25, cond: 'Hujan Ringan', cond_id: 'Hujan Ringan', cond_en: 'Light Rain', icon: '🌦️', aqi: 35, hourly: [
      { t: '12:00', icon: '🌧️', temp: 27 },
      { t: '15:00', icon: '⛅', temp: 29 },
      { t: '18:00', icon: '🌦️', temp: 27 },
      { t: '21:00', icon: '🌙', temp: 26 }
    ]}
  };

  const DokuWidgets = {
    containerEl: null,
    activeWidgets: ['clock', 'calendar', 'weather', 'resources', 'stickynote'],
    isVisible: true,
    calendarOffset: 0,
    currentCity: 'Jakarta',
    clockTimer: null,
    metricsTimer: null,
    systemMetrics: {
      cpu_percent: 12.5,
      memory_used_mb: 1420,
      memory_total_mb: 8192,
      memory_percent: 17.3,
      storage_used_mb: 5.1,
      storage_quota_mb: 30720,
      storage_percent: 0.1
    },

    init() {
      // 1. Load active widgets preference
      const savedActive = localStorage.getItem('dokuhub_active_widgets');
      if (savedActive) {
        try {
          this.activeWidgets = JSON.parse(savedActive);
        } catch (e) {}
      }

      const savedVis = localStorage.getItem('dokuhub_widgets_visible');
      if (savedVis !== null) {
        this.isVisible = savedVis === 'true';
      }

      const savedCity = localStorage.getItem('dokuhub_widget_city');
      if (savedCity && WEATHER_PRESETS[savedCity]) {
        this.currentCity = savedCity;
      }

      // 2. Locate or create container in DOM
      this.ensureContainer();

      // 3. Render widgets
      this.render();

      // 4. Start Clock loop
      this.startClockLoop();

      // 5. Fetch initial metrics and loop
      this.fetchMetrics();
      this.metricsTimer = setInterval(() => this.fetchMetrics(), 15000);
    },

    ensureContainer() {
      let layer = document.getElementById('desktop-widgets-layer');
      if (!layer) {
        layer = document.createElement('div');
        layer.id = 'desktop-widgets-layer';
        layer.className = 'desktop-widgets-layer';

        const desktop = document.getElementById('desktop-screen');
        const windowSpace = document.getElementById('window-space');
        if (desktop && windowSpace) {
          desktop.insertBefore(layer, windowSpace);
        } else if (desktop) {
          desktop.appendChild(layer);
        } else {
          document.body.appendChild(layer);
        }
      }
      this.containerEl = layer;
    },

    toggleVisibility() {
      this.isVisible = !this.isVisible;
      localStorage.setItem('dokuhub_widgets_visible', String(this.isVisible));
      if (this.containerEl) {
        if (this.isVisible) {
          this.containerEl.classList.remove('hidden');
          this.render();
          if (window.DokuIsland) {
            const isId = (window.DokuI18n && DokuI18n.lang === 'id');
            DokuIsland.info(isId ? 'Widget desktop ditampilkan' : 'Desktop widgets shown', isId ? 'Widget macOS' : 'macOS Widget');
          }
        } else {
          this.containerEl.classList.add('hidden');
          if (window.DokuIsland) {
            const isId = (window.DokuI18n && DokuI18n.lang === 'id');
            DokuIsland.info(isId ? 'Widget desktop disembunyikan' : 'Desktop widgets hidden', isId ? 'Widget macOS' : 'macOS Widget');
          }
        }
      }
      this.updateMenubarIcon();
    },

    updateMenubarIcon() {
      const btn = document.getElementById('menubar-widget-toggle');
      if (btn) {
        btn.classList.toggle('active', this.isVisible);
        btn.title = this.isVisible ? 'Sembunyikan Widget Desktop' : 'Tampilkan Widget Desktop';
      }
    },

    startClockLoop() {
      if (this.clockTimer) clearInterval(this.clockTimer);
      const updateHands = () => {
        if (!this.isVisible) return;
        const now = new Date();
        const sec = now.getSeconds();
        const min = now.getMinutes();
        const hr = now.getHours();

        const secDeg = (sec / 60) * 360;
        const minDeg = ((min + sec / 60) / 60) * 360;
        const hrDeg = (((hr % 12) + min / 60) / 12) * 360;

        const secEl = document.getElementById('widget-clock-sec');
        const minEl = document.getElementById('widget-clock-min');
        const hrEl = document.getElementById('widget-clock-hr');
        const digEl = document.getElementById('widget-clock-digital');

        if (secEl) secEl.style.transform = `rotate(${secDeg}deg)`;
        if (minEl) minEl.style.transform = `rotate(${minDeg}deg)`;
        if (hrEl) hrEl.style.transform = `rotate(${hrDeg}deg)`;
        if (digEl) {
          const hh = String(hr).padStart(2, '0');
          const mm = String(min).padStart(2, '0');
          const ss = String(sec).padStart(2, '0');
          digEl.textContent = `${hh}:${mm}:${ss}`;
        }
      };

      updateHands();
      this.clockTimer = setInterval(updateHands, 1000);
    },

    async fetchMetrics() {
      try {
        if (window.DokuTahoeLogin && DokuTahoeLogin.isLocked) return;
        const headers = {};
        const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
        if (!token) return;
        headers['Authorization'] = `Bearer ${token}`;
        const curUser = (window.DokuHub && DokuHub.currentUser) || null;
        if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

        const [metricsRes, quotaRes] = await Promise.allSettled([
          fetch('/api/system/metrics', { headers }),
          fetch('/api/storage/quota', { headers })
        ]);

        if (metricsRes.status === 'fulfilled' && metricsRes.value.ok) {
          const data = await metricsRes.value.json();
          if (data.cpu) {
            this.systemMetrics.cpu_percent = data.cpu.usage_percent || 12;
          }
          if (data.memory) {
            this.systemMetrics.memory_used_mb = (data.memory.used_bytes || 0) / (1024 * 1024);
            this.systemMetrics.memory_total_mb = (data.memory.total_bytes || 0) / (1024 * 1024);
            this.systemMetrics.memory_percent = data.memory.used_percent || 15;
          }
        }

        if (quotaRes.status === 'fulfilled' && quotaRes.value.ok) {
          const qData = await quotaRes.value.json();
          this.systemMetrics.storage_used_mb = qData.used_mb || 0;
          this.systemMetrics.storage_quota_mb = qData.quota_mb || 30720;
          this.systemMetrics.storage_percent = qData.percent_used || 0;
        }

        this.updateResourceWidgetDOM();
      } catch (e) {}
    },

    render() {
      if (!this.containerEl) return;
      if (!this.isVisible) {
        this.containerEl.classList.add('hidden');
        return;
      }
      this.containerEl.classList.remove('hidden');

      const isId = (window.DokuI18n && DokuI18n.lang === 'id');

      let html = `
        <div class="widgets-header-bar">
          <div class="widgets-badge">
            <span>◫</span>
            <span>${isId ? 'Widget Desktop macOS' : 'macOS Desktop Widgets'}</span>
          </div>
          <button class="widgets-manage-btn" onclick="DokuWidgets.openManageModal()" title="${isId ? 'Kelola Widget' : 'Manage Widgets'}">
            ⚙️ ${isId ? 'Atur' : 'Edit'}
          </button>
        </div>
        <div class="widgets-grid-container">
      `;

      this.activeWidgets.forEach(wId => {
        if (wId === 'clock') html += this.renderClockHtml(isId);
        else if (wId === 'calendar') html += this.renderCalendarHtml(isId);
        else if (wId === 'weather') html += this.renderWeatherHtml(isId);
        else if (wId === 'resources') html += this.renderResourcesHtml(isId);
        else if (wId === 'stickynote') html += this.renderStickyNoteHtml(isId);
      });

      html += `</div>`;
      this.containerEl.innerHTML = html;
      this.updateMenubarIcon();
    },

    /* ------------------------------------------------------------------------
       1. Clock Widget
       ------------------------------------------------------------------------ */
    renderClockHtml(isId) {
      const now = new Date();
      const dayNamesId = ['Minggu', 'Senin', 'Selasa', 'Rabu', 'Kamis', 'Jumat', 'Sabtu'];
      const dayNamesEn = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
      const dayName = isId ? dayNamesId[now.getDay()] : dayNamesEn[now.getDay()];

      return `
        <div class="mac-widget-card widget-clock" id="widget-card-clock">
          <div class="widget-header">
            <span class="widget-title-icon">🕒</span>
            <span class="widget-title">${isId ? 'Jam Dunia & Waktu Lokal' : 'World & Local Clock'}</span>
            <span class="widget-badge-sub">${isId ? 'WIB • UTC+7' : 'UTC+7 • Jakarta'}</span>
          </div>
          <div class="widget-clock-body">
            <!-- Analog Clock Face -->
            <div class="analog-clock-wrap">
              <svg class="analog-clock-svg" viewBox="0 0 100 100">
                <circle cx="50" cy="50" r="46" class="clock-dial" />
                <!-- 12 Hour Ticks -->
                ${[0, 30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330].map(deg => `
                  <line x1="50" y1="9" x2="50" y2="${deg % 90 === 0 ? '15' : '12'}" 
                        class="clock-tick ${deg % 90 === 0 ? 'major' : ''}" 
                        transform="rotate(${deg} 50 50)" />
                `).join('')}
                <!-- Clock Hands -->
                <line x1="50" y1="50" x2="50" y2="24" id="widget-clock-hr" class="clock-hand hand-hour" />
                <line x1="50" y1="50" x2="50" y2="15" id="widget-clock-min" class="clock-hand hand-min" />
                <line x1="50" y1="54" x2="50" y2="12" id="widget-clock-sec" class="clock-hand hand-sec" />
                <circle cx="50" cy="50" r="3" class="clock-pin" />
              </svg>
            </div>
            <!-- Digital Clock & Details -->
            <div class="digital-clock-details">
              <div class="digital-time-display" id="widget-clock-digital">--:--:--</div>
              <div class="digital-day-label">${dayName}, ${now.getDate()}</div>
              <div class="digital-city-label">🇮🇩 Jakarta (WIB)</div>
              <div class="digital-subinfo">${isId ? 'Matahari terbit 05:28 • Terbenam 17:52' : 'Sunrise 05:28 • Sunset 17:52'}</div>
            </div>
          </div>
        </div>
      `;
    },

    /* ------------------------------------------------------------------------
       2. Calendar Widget
       ------------------------------------------------------------------------ */
    renderCalendarHtml(isId) {
      const now = new Date();
      const viewDate = new Date(now.getFullYear(), now.getMonth() + this.calendarOffset, 1);
      const year = viewDate.getFullYear();
      const month = viewDate.getMonth();

      const monthNamesId = ['Januari', 'Februari', 'Maret', 'April', 'Mei', 'Juni', 'Juli', 'Agustus', 'September', 'Oktober', 'November', 'Desember'];
      const monthNamesEn = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
      const monthLabel = isId ? monthNamesId[month] : monthNamesEn[month];

      const dayLettersId = ['M', 'S', 'S', 'R', 'K', 'J', 'S'];
      const dayLettersEn = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];
      const dayHeaders = isId ? dayLettersId : dayLettersEn;

      // Calculate calendar grid days
      const firstDayIndex = new Date(year, month, 1).getDay();
      const totalDays = new Date(year, month + 1, 0).getDate();
      const prevTotalDays = new Date(year, month, 0).getDate();

      let daysHtml = '';
      // Prev month filler
      for (let x = firstDayIndex; x > 0; x--) {
        daysHtml += `<span class="cal-day other-month">${prevTotalDays - x + 1}</span>`;
      }
      // Current month days
      for (let i = 1; i <= totalDays; i++) {
        const isToday = (this.calendarOffset === 0 && i === now.getDate());
        daysHtml += `<span class="cal-day ${isToday ? 'today' : ''}">${i}</span>`;
      }

      return `
        <div class="mac-widget-card widget-calendar" id="widget-card-calendar">
          <div class="widget-header">
            <span class="widget-title-icon">📅</span>
            <span class="widget-title">${isId ? 'Kalender & Agenda' : 'Calendar & Schedule'}</span>
            <div class="cal-nav-buttons">
              <button onclick="DokuWidgets.navCalendar(-1)" title="${isId ? 'Bulan Sebelumnya' : 'Previous Month'}">‹</button>
              <button onclick="DokuWidgets.navCalendar(0)" title="${isId ? 'Hari Ini' : 'Today'}">•</button>
              <button onclick="DokuWidgets.navCalendar(1)" title="${isId ? 'Bulan Selanjutnya' : 'Next Month'}">›</button>
            </div>
          </div>
          <div class="widget-calendar-body">
            <!-- Left Big Date Banner -->
            <div class="cal-big-banner">
              <div class="cal-big-month">${monthLabel.slice(0, 3).toUpperCase()}</div>
              <div class="cal-big-number">${now.getDate()}</div>
              <div class="cal-big-year">${year}</div>
            </div>
            <!-- Right Month Grid -->
            <div class="cal-grid-section">
              <div class="cal-month-header">${monthLabel} ${year}</div>
              <div class="cal-weekdays-row">
                ${dayHeaders.map(d => `<span class="cal-weekday">${d}</span>`).join('')}
              </div>
              <div class="cal-days-grid">
                ${daysHtml}
              </div>
            </div>
          </div>
          <!-- Agenda Footer -->
          <div class="cal-agenda-footer">
            <span class="agenda-dot"></span>
            <span class="agenda-text">📌 ${isId ? 'BijiOS Multi-Tenant Workspace Aktif' : 'BijiOS Active Multi-Tenant Workspace'}</span>
          </div>
        </div>
      `;
    },

    navCalendar(step) {
      if (step === 0) {
        this.calendarOffset = 0;
      } else {
        this.calendarOffset += step;
      }
      this.render();
    },

    /* ------------------------------------------------------------------------
       3. Weather Widget
       ------------------------------------------------------------------------ */
    renderWeatherHtml(isId) {
      const data = WEATHER_PRESETS[this.currentCity] || WEATHER_PRESETS['Jakarta'];

      return `
        <div class="mac-widget-card widget-weather" id="widget-card-weather">
          <div class="widget-header">
            <span class="widget-title-icon">🌤️</span>
            <span class="widget-title">${isId ? 'Cuaca & Suhu' : 'Weather'}</span>
            <!-- City Cycle Button -->
            <button class="city-selector-btn" onclick="DokuWidgets.cycleCity()" title="${isId ? 'Ganti Kota' : 'Switch City'}">
              📍 ${this.currentCity} ▾
            </button>
          </div>
          <div class="widget-weather-body">
            <div class="weather-current-row">
              <div class="weather-icon-large">${data.icon}</div>
              <div class="weather-temp-wrap">
                <div class="weather-temp-main">${data.temp}°</div>
                <div class="weather-condition-text">${isId ? (data.cond_id || data.cond) : (data.cond_en || data.cond)}</div>
              </div>
              <div class="weather-hi-low">
                <div class="hi-tag">${isId ? 'T' : 'H'}: ${data.high}°</div>
                <div class="lo-tag">${isId ? 'R' : 'L'}: ${data.low}°</div>
                <div class="aqi-badge">AQI ${data.aqi}</div>
              </div>
            </div>
            <!-- Hourly Forecast Pills -->
            <div class="weather-hourly-row">
              ${data.hourly.map(h => `
                <div class="hourly-forecast-item">
                  <span class="hourly-time">${h.t}</span>
                  <span class="hourly-icon">${h.icon}</span>
                  <span class="hourly-temp">${h.temp}°</span>
                </div>
              `).join('')}
            </div>
          </div>
        </div>
      `;
    },

    cycleCity() {
      const cities = Object.keys(WEATHER_PRESETS);
      const curIdx = cities.indexOf(this.currentCity);
      const nextCity = cities[(curIdx + 1) % cities.length];
      this.currentCity = nextCity;
      localStorage.setItem('dokuhub_widget_city', nextCity);
      this.render();
      if (window.DokuIsland) {
        DokuIsland.info(isId ? `Lokasi cuaca diubah ke ${nextCity}` : `Weather location changed to ${nextCity}`, isId ? 'Cuaca' : 'Weather');
      }
    },

    /* ------------------------------------------------------------------------
       4. System Resources Radial Gauges Widget
       ------------------------------------------------------------------------ */
    renderResourcesHtml(isId) {
      const m = this.systemMetrics;
      const cpu = Math.round(m.cpu_percent);
      const ram = Math.round(m.memory_percent);
      const stg = Math.min(100, Math.round(m.storage_percent || 0.1));

      return `
        <div class="mac-widget-card widget-resources" id="widget-card-resources">
          <div class="widget-header">
            <span class="widget-title-icon">⚡</span>
            <span class="widget-title">${isId ? 'Ringkasan Sistem & Storage' : 'System & Storage'}</span>
            <button class="res-refresh-btn" onclick="DokuWidgets.fetchMetrics()" title="${isId ? 'Perbarui metrik' : 'Refresh metrics'}">↻</button>
          </div>
          <div class="widget-resources-body">
            <!-- CPU Gauge -->
            <div class="radial-gauge-item">
              ${this.renderRadialSvg(cpu, '#007aff', 'cpu')}
              <div class="gauge-label">CPU</div>
              <div class="gauge-sub">${cpu}% load</div>
            </div>
            <!-- RAM Gauge -->
            <div class="radial-gauge-item">
              ${this.renderRadialSvg(ram, '#af52de', 'ram')}
              <div class="gauge-label">Memory</div>
              <div class="gauge-sub">${(m.memory_used_mb / 1024).toFixed(1)} GB</div>
            </div>
            <!-- User Storage Gauge -->
            <div class="radial-gauge-item">
              ${this.renderRadialSvg(stg, '#34c759', 'stg')}
              <div class="gauge-label">${isId ? 'Kapasitas' : 'Storage'}</div>
              <div class="gauge-sub">${m.storage_used_mb.toFixed(1)} MB</div>
            </div>
          </div>
          <div class="resources-footer-note">
            Pure Rust Kernel Veyla • TOON v4 Engine • Zero-Copy Memory
          </div>
        </div>
      `;
    },

    renderRadialSvg(percent, color, id) {
      const radius = 24;
      const circumference = 2 * Math.PI * radius; // ~150.79
      const strokeDash = ((100 - percent) / 100) * circumference;

      return `
        <div class="gauge-svg-wrap">
          <svg class="gauge-svg" width="60" height="60" viewBox="0 0 60 60">
            <circle cx="30" cy="30" r="${radius}" class="gauge-track" />
            <circle cx="30" cy="30" r="${radius}" class="gauge-fill" 
                    id="gauge-fill-${id}"
                    stroke="${color}"
                    stroke-dasharray="${circumference}"
                    stroke-dashoffset="${strokeDash}" />
          </svg>
          <span class="gauge-center-text" id="gauge-text-${id}">${percent}%</span>
        </div>
      `;
    },

    updateResourceWidgetDOM() {
      const m = this.systemMetrics;
      const cpu = Math.round(m.cpu_percent);
      const ram = Math.round(m.memory_percent);
      const stg = Math.min(100, Math.round(m.storage_percent || 0.1));

      const radius = 24;
      const circumference = 2 * Math.PI * radius;

      const updateGauge = (id, pct) => {
        const fill = document.getElementById(`gauge-fill-${id}`);
        const text = document.getElementById(`gauge-text-${id}`);
        if (fill) fill.style.strokeDashoffset = ((100 - pct) / 100) * circumference;
        if (text) text.textContent = `${pct}%`;
      };

      updateGauge('cpu', cpu);
      updateGauge('ram', ram);
      updateGauge('stg', stg);
    },

    /* ------------------------------------------------------------------------
       5. Sticky Note Widget (Catatan Cepat)
       ------------------------------------------------------------------------ */
    renderStickyNoteHtml(isId) {
      const curUser = (window.DokuHub && DokuHub.currentUser) || { id: 'default' };
      const noteKey = `dokuhub_widget_note_${curUser.id}`;
      const savedNote = localStorage.getItem(noteKey) || (isId ? '💡 Ide & Catatan Cepat:\n- Selesaikan dokumen laporan di WordCraft\n- Periksa formula spreadsheet di GriidCraft' : '💡 Quick Thoughts:\n- Finish team doc in WordCraft\n- Review budget in GriidCraft');

      return `
        <div class="mac-widget-card widget-stickynote" id="widget-card-stickynote">
          <div class="widget-header">
            <span class="widget-title-icon">📝</span>
            <span class="widget-title">${isId ? 'Catatan Cepat Desktop' : 'Quick Sticky Note'}</span>
            <span class="sticky-pin">📌</span>
          </div>
          <div class="widget-stickynote-body">
            <textarea class="sticky-textarea" id="widget-sticky-textarea" 
                      placeholder="${isId ? 'Tulis catatan di sini...' : 'Type note here...'}"
                      oninput="DokuWidgets.saveStickyNote(this.value)">${savedNote}</textarea>
          </div>
          <div class="sticky-footer">
            <span id="sticky-save-indicator">${isId ? '✓ Tersimpan otomatis' : '✓ Auto-saved'}</span>
            <button class="sticky-clear-btn" onclick="DokuWidgets.clearStickyNote()" title="${isId ? 'Kosongkan catatan' : 'Clear note'}">✕</button>
          </div>
        </div>
      `;
    },

    saveStickyNote(val) {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      const curUser = (window.DokuHub && DokuHub.currentUser) || { id: 'default' };
      localStorage.setItem(`dokuhub_widget_note_${curUser.id}`, val);
      const ind = document.getElementById('sticky-save-indicator');
      if (ind) {
        ind.textContent = isId ? 'Menyimpan...' : 'Saving...';
        setTimeout(() => {
          if (ind) ind.textContent = isId ? '✓ Tersimpan' : '✓ Saved';
        }, 300);
      }
    },

    clearStickyNote() {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      const curUser = (window.DokuHub && DokuHub.currentUser) || { id: 'default' };
      localStorage.setItem(`dokuhub_widget_note_${curUser.id}`, '');
      const area = document.getElementById('widget-sticky-textarea');
      if (area) area.value = '';
      if (window.DokuIsland) {
        DokuIsland.info(isId ? 'Catatan sticky telah dikosongkan' : 'Sticky note cleared', isId ? 'Catatan Tempel' : 'Sticky Note');
      }
    },

    /* ------------------------------------------------------------------------
       Manage Widgets Modal (Galeri Widget)
       ------------------------------------------------------------------------ */
    openManageModal() {
      let modal = document.getElementById('modal-manage-widgets');
      if (!modal) {
        modal = document.createElement('div');
        modal.id = 'modal-manage-widgets';
        modal.className = 'widgets-modal-overlay';
        document.body.appendChild(modal);
      }

      const isId = (window.DokuI18n && DokuI18n.lang === 'id');

      const allWidgetDefs = [
        { id: 'clock', icon: '🕒', name: isId ? 'Jam Dunia & Analog' : 'World & Analog Clock', desc: isId ? 'Jam jarum analog, waktu digital detik, dan informasi zona waktu.' : 'Analog dial, live ticking hands, and UTC timezone badge.' },
        { id: 'calendar', icon: '📅', name: isId ? 'Kalender & Agenda Bulanan' : 'Interactive Calendar', desc: isId ? 'Navigasi bulan, penanda tanggal hari ini, dan jadwal agenda.' : 'Month navigation, current day highlight, and agenda snippet.' },
        { id: 'weather', icon: '🌤️', name: isId ? 'Cuaca & Suhu Kota' : 'Live Weather & Forecast', desc: isId ? 'Suhu saat ini, ramalan per jam, dan pemilih kota nusantara.' : 'Current temperature, hourly forecast, and city switcher.' },
        { id: 'resources', icon: '⚡', name: isId ? 'Penggunaan CPU & Storage' : 'System Resources Monitor', desc: isId ? 'Radial gauge beban prosesor, RAM server, dan kuota disk personal.' : 'Radial gauges for CPU load, RAM memory, and user storage.' },
        { id: 'stickynote', icon: '📝', name: isId ? 'Catatan Tempel (Sticky Note)' : 'Quick Glass Sticky Note', desc: isId ? 'Catatan cepat kuning amber dengan penyimpanan otomatis di desktop.' : 'Amber glass notepad with real-time auto-saving.' }
      ];

      modal.innerHTML = `
        <div class="widgets-modal-card">
          <div class="widgets-modal-header">
            <div>
              <h2 style="font-size:18px; font-weight:700; margin:0 0 4px 0;">◫ ${isId ? 'Galeri Widget Desktop macOS' : 'macOS Desktop Widget Gallery'}</h2>
              <p style="font-size:12px; color:var(--text-muted); margin:0;">${isId ? 'Aktifkan atau nonaktifkan widget yang tampil di desktop Anda.' : 'Choose which widgets to display on your desktop surface.'}</p>
            </div>
            <button class="traffic-btn btn-close" onclick="DokuWidgets.closeManageModal()"></button>
          </div>

          <div class="widgets-modal-body">
            ${allWidgetDefs.map(w => {
              const isActive = this.activeWidgets.includes(w.id);
              return `
                <div class="widget-toggle-row">
                  <div class="widget-row-left">
                    <span class="widget-row-icon">${w.icon}</span>
                    <div>
                      <div class="widget-row-name">${w.name}</div>
                      <div class="widget-row-desc">${w.desc}</div>
                    </div>
                  </div>
                  <label class="mac-switch">
                    <input type="checkbox" ${isActive ? 'checked' : ''} onchange="DokuWidgets.toggleWidget('${w.id}', this.checked)">
                    <span class="mac-slider"></span>
                  </label>
                </div>
              `;
            }).join('')}
          </div>

          <div class="widgets-modal-footer">
            <button class="mac-btn" onclick="DokuWidgets.toggleVisibility()">
              ${this.isVisible ? (isId ? '👁️ Sembunyikan Semua Widget' : '👁️ Hide All Widgets') : (isId ? '👁️ Tampilkan Semua Widget' : '👁️ Show All Widgets')}
            </button>
            <button class="mac-btn primary" onclick="DokuWidgets.closeManageModal()">
              ${isId ? 'Selesai' : 'Done'}
            </button>
          </div>
        </div>
      `;

      modal.style.display = 'flex';
    },

    closeManageModal() {
      const modal = document.getElementById('modal-manage-widgets');
      if (modal) modal.style.display = 'none';
    },

    toggleWidget(widgetId, isChecked) {
      if (isChecked) {
        if (!this.activeWidgets.includes(widgetId)) {
          this.activeWidgets.push(widgetId);
        }
      } else {
        this.activeWidgets = this.activeWidgets.filter(id => id !== widgetId);
      }
      localStorage.setItem('dokuhub_active_widgets', JSON.stringify(this.activeWidgets));
      this.render();
      if (window.DokuIsland) {
        const isId = (window.DokuI18n && DokuI18n.lang === 'id');
        DokuIsland.success(isId ? 'Pengaturan widget diperbarui' : 'Widget preferences updated', isId ? 'Widget Desktop' : 'Desktop Widget');
      }
    }
  };

  // Expose to window
  window.DokuWidgets = DokuWidgets;

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => DokuWidgets.init());
  } else {
    DokuWidgets.init();
  }
})();
