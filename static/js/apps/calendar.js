/**
 * DokuCalendar - macOS-style Calendar & Event Schedule Application
 * Features:
 * - Monthly grid view with Apple Calendar red today indicator
 * - Interactive event creation, categorization (Work, Personal, Meeting, Urgent), and deletion
 * - Selected day agenda sidebar with persistent localStorage storage per user
 * - 100% Bilingual (ID & EN) & Adaptive Light/Dark Mode
 */

window.DokuCalendar = {
  container: null,
  currentYear: new Date().getFullYear(),
  currentMonth: new Date().getMonth(),
  selectedDateStr: null,
  events: [],

  init(containerEl) {
    this.container = containerEl;
    const now = new Date();
    this.currentYear = now.getFullYear();
    this.currentMonth = now.getMonth();
    this.selectedDateStr = this.formatDateKey(now.getFullYear(), now.getMonth(), now.getDate());
    this.loadEvents();
    this.render();
  },

  getStorageKey() {
    const uid = (window.DokuHub && DokuHub.currentUser && DokuHub.currentUser.id) ? DokuHub.currentUser.id : 'default';
    return `dokuhub_calendar_events_${uid}`;
  },

  loadEvents() {
    try {
      const saved = localStorage.getItem(this.getStorageKey());
      if (saved) {
        this.events = JSON.parse(saved);
        return;
      }
    } catch { }
    // Seed 1 helpful welcome event on today's date if empty
    const todayKey = this.formatDateKey(new Date().getFullYear(), new Date().getMonth(), new Date().getDate());
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    this.events = [
      {
        id: 'evt_welcome',
        date: todayKey,
        title: isId ? 'Sinkronisasi Tim BijiOS' : 'BijiOS Team Sync',
        time: '10:00',
        category: 'meeting',
        notes: isId ? 'Rapat koordinasi mingguan melalui BijiMeet.' : 'Weekly coordination via BijiMeet.'
      }
    ];
    this.saveEvents();
  },

  saveEvents() {
    try {
      localStorage.setItem(this.getStorageKey(), JSON.stringify(this.events));
    } catch { }
  },

  formatDateKey(y, m, d) {
    return `${y}-${String(m + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
  },

  getCategoryColor(cat) {
    switch (cat) {
      case 'work': return '#007aff';
      case 'personal': return '#10b981';
      case 'meeting': return '#8b5cf6';
      case 'urgent': return '#ef4444';
      default: return '#007aff';
    }
  },

  prevMonth() {
    this.currentMonth--;
    if (this.currentMonth < 0) {
      this.currentMonth = 11;
      this.currentYear--;
    }
    this.render();
  },

  nextMonth() {
    this.currentMonth++;
    if (this.currentMonth > 11) {
      this.currentMonth = 0;
      this.currentYear++;
    }
    this.render();
  },

  goToday() {
    const now = new Date();
    this.currentYear = now.getFullYear();
    this.currentMonth = now.getMonth();
    this.selectedDateStr = this.formatDateKey(now.getFullYear(), now.getMonth(), now.getDate());
    this.render();
  },

  selectDate(dateStr) {
    this.selectedDateStr = dateStr;
    this.render();
  },

  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    const monthNames = isId
      ? ['Januari', 'Februari', 'Maret', 'April', 'Mei', 'Juni', 'Juli', 'Agustus', 'September', 'Oktober', 'November', 'Desember']
      : ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

    const dayNames = isId
      ? ['Min', 'Sen', 'Sel', 'Rab', 'Kam', 'Jum', 'Sab']
      : ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

    const now = new Date();
    const todayKey = this.formatDateKey(now.getFullYear(), now.getMonth(), now.getDate());

    const firstDay = new Date(this.currentYear, this.currentMonth, 1).getDay();
    const daysInMonth = new Date(this.currentYear, this.currentMonth + 1, 0).getDate();
    const prevMonthDays = new Date(this.currentYear, this.currentMonth, 0).getDate();

    const cells = [];
    // Leading days from previous month
    for (let i = firstDay - 1; i >= 0; i--) {
      cells.push({ day: prevMonthDays - i, otherMonth: true, dateKey: null });
    }
    // Current month days
    for (let d = 1; d <= daysInMonth; d++) {
      cells.push({
        day: d,
        otherMonth: false,
        dateKey: this.formatDateKey(this.currentYear, this.currentMonth, d)
      });
    }
    // Trailing days to complete grid
    let nextDay = 1;
    while (cells.length % 7 !== 0) {
      cells.push({ day: nextDay++, otherMonth: true, dateKey: null });
    }

    const selectedEvents = this.events
      .filter(e => e.date === this.selectedDateStr)
      .sort((a, b) => (a.time || '').localeCompare(b.time || ''));

    this.container.innerHTML = `
      <div style="display:flex; height:100%; background:var(--window-bg); color:var(--text-main); font-family:-apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif; overflow:hidden;">
        <!-- Left Sidebar: Categories & Selected Day Agenda -->
        <div style="width:250px; background:var(--sidebar-bg); border-right:1px solid var(--window-border); padding:16px 14px; display:flex; flex-direction:column; flex-shrink:0; overflow-y:auto;">
          <div style="display:flex; align-items:center; justify-content:space-between; margin-bottom:14px;">
            <strong style="font-size:14px;">📅 ${isId ? 'Agenda' : 'Schedule'}</strong>
            <button class="mac-btn primary" onclick="DokuCalendar.openAddEventModal()" style="padding:4px 10px; font-size:11px;">
              + ${isId ? 'Acara' : 'Event'}
            </button>
          </div>

          <div style="font-size:11.5px; font-weight:700; color:var(--accent-color); margin-bottom:10px; padding:6px 10px; background:rgba(0,122,255,0.1); border-radius:8px;">
            ${this.selectedDateStr}
          </div>

          <!-- Selected Day Events -->
          <div style="flex:1; display:flex; flex-direction:column; gap:8px; margin-bottom:16px;">
            ${selectedEvents.length === 0 ? `
              <div style="padding:18px 10px; text-align:center; font-size:12px; color:var(--text-muted); border:1px dashed var(--window-border); border-radius:10px;">
                ${isId ? 'Tidak ada jadwal pada tanggal ini.' : 'No events scheduled for this day.'}
              </div>
            ` : selectedEvents.map(ev => `
              <div style="padding:10px; border-radius:10px; background:var(--card-bg); border:1px solid var(--window-border); border-left:4px solid ${this.getCategoryColor(ev.category)}; position:relative;">
                <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:6px;">
                  <div style="font-size:12.5px; font-weight:700; color:var(--text-main);">${ev.title}</div>
                  <button onclick="DokuCalendar.deleteEvent('${ev.id}')" style="background:none; border:none; color:var(--text-muted); cursor:pointer; font-size:11px; padding:0 2px;" title="${isId ? 'Hapus' : 'Delete'}">✕</button>
                </div>
                ${ev.time ? `<div style="font-size:11px; color:var(--text-muted); margin-top:2px;">🕒 ${ev.time}</div>` : ''}
                ${ev.notes ? `<div style="font-size:11px; color:var(--text-muted); margin-top:4px; line-height:1.4;">${ev.notes}</div>` : ''}
              </div>
            `).join('')}
          </div>

          <!-- Legend -->
          <div style="border-top:1px solid var(--window-border); padding-top:12px; font-size:11px; color:var(--text-muted); display:grid; grid-template-columns:1fr 1fr; gap:6px;">
            <div style="display:flex; align-items:center; gap:6px;"><span style="width:8px; height:8px; border-radius:50%; background:#007aff;"></span>${isId ? 'Pekerjaan' : 'Work'}</div>
            <div style="display:flex; align-items:center; gap:6px;"><span style="width:8px; height:8px; border-radius:50%; background:#10b981;"></span>${isId ? 'Pribadi' : 'Personal'}</div>
            <div style="display:flex; align-items:center; gap:6px;"><span style="width:8px; height:8px; border-radius:50%; background:#8b5cf6;"></span>${isId ? 'Rapat' : 'Meeting'}</div>
            <div style="display:flex; align-items:center; gap:6px;"><span style="width:8px; height:8px; border-radius:50%; background:#ef4444;"></span>${isId ? 'Penting' : 'Urgent'}</div>
          </div>
        </div>

        <!-- Main Calendar Area -->
        <div style="flex:1; display:flex; flex-direction:column; min-width:0; overflow:hidden;">
          <!-- Month Header -->
          <div style="padding:12px 18px; background:var(--window-header-bg); border-bottom:1px solid var(--window-border); display:flex; align-items:center; justify-content:space-between;">
            <h2 style="font-size:18px; font-weight:800; margin:0;">
              ${monthNames[this.currentMonth]} <span style="font-weight:400; color:var(--text-muted);">${this.currentYear}</span>
            </h2>
            <div style="display:flex; align-items:center; gap:6px;">
              <button class="mac-btn" onclick="DokuCalendar.prevMonth()" style="padding:4px 10px;">‹</button>
              <button class="mac-btn" onclick="DokuCalendar.goToday()" style="padding:4px 12px; font-size:12px; font-weight:600;">
                ${isId ? 'Hari Ini' : 'Today'}
              </button>
              <button class="mac-btn" onclick="DokuCalendar.nextMonth()" style="padding:4px 10px;">›</button>
            </div>
          </div>

          <!-- Day Names Row -->
          <div style="display:grid; grid-template-columns:repeat(7, 1fr); border-bottom:1px solid var(--window-border); background:var(--window-header-bg); font-size:11px; font-weight:700; color:var(--text-muted); text-align:right; padding:6px 0;">
            ${dayNames.map((dn, idx) => `<div style="padding:0 10px; ${idx === 0 ? 'color:#ef4444;' : ''}">${dn}</div>`).join('')}
          </div>

          <!-- Month Grid -->
          <div style="flex:1; display:grid; grid-template-columns:repeat(7, 1fr); grid-auto-rows:1fr; overflow-y:auto;">
            ${cells.map(c => {
      const isToday = c.dateKey === todayKey;
      const isSelected = c.dateKey && c.dateKey === this.selectedDateStr;
      const dayEvents = c.dateKey ? this.events.filter(e => e.date === c.dateKey) : [];

      return `
                <div onclick="${c.dateKey ? `DokuCalendar.selectDate('${c.dateKey}')` : ''}"
                     ondblclick="${c.dateKey ? `DokuCalendar.selectDate('${c.dateKey}'); DokuCalendar.openAddEventModal();` : ''}"
                     style="border-right:1px solid var(--window-border); border-bottom:1px solid var(--window-border); padding:6px; cursor:${c.dateKey ? 'pointer' : 'default'}; background:${isSelected ? 'rgba(0,122,255,0.08)' : (c.otherMonth ? 'rgba(125,125,125,0.04)' : 'transparent')}; display:flex; flex-direction:column; gap:3px; min-height:70px; transition:background 0.15s;">
                  <div style="display:flex; justify-content:flex-end;">
                    <span style="width:24px; height:24px; border-radius:50%; display:inline-flex; align-items:center; justify-content:center; font-size:12px; font-weight:${isToday || isSelected ? '700' : '500'}; background:${isToday ? '#ff3b30' : 'transparent'}; color:${isToday ? '#fff' : (c.otherMonth ? 'var(--text-muted)' : 'var(--text-main)')}; opacity:${c.otherMonth ? '0.45' : '1'};">
                      ${c.day}
                    </span>
                  </div>
                  <div style="display:flex; flex-direction:column; gap:2px; overflow:hidden;">
                    ${dayEvents.slice(0, 3).map(ev => `
                      <div style="font-size:10px; font-weight:600; padding:2px 5px; border-radius:4px; background:${this.getCategoryColor(ev.category)}; color:#fff; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">
                        ${ev.time ? ev.time + ' ' : ''}${ev.title}
                      </div>
                    `).join('')}
                    ${dayEvents.length > 3 ? `<div style="font-size:9.5px; color:var(--text-muted); padding-left:4px;">+${dayEvents.length - 3} more</div>` : ''}
                  </div>
                </div>
              `;
    }).join('')}
          </div>
        </div>
      </div>
    `;
  },

  openAddEventModal() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    let modal = document.getElementById('doku-calendar-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'doku-calendar-modal';
      modal.style.cssText = 'position:fixed; inset:0; background:rgba(0,0,0,0.4); backdrop-filter:blur(12px); z-index:999999; display:none; align-items:center; justify-content:center;';
      document.body.appendChild(modal);
    }

    modal.style.display = 'flex';
    modal.innerHTML = `
      <div style="width:420px; max-width:92vw; background:var(--window-bg); border:1px solid var(--window-border); border-radius:16px; box-shadow:var(--window-shadow); padding:22px; color:var(--text-main); animation:macModalPopIn 0.22s cubic-bezier(0.16,1,0.3,1);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <h3 style="font-size:16px; font-weight:700; margin:0;">📅 ${isId ? 'Tambah Acara Baru' : 'New Calendar Event'}</h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('doku-calendar-modal').style.display='none'"></button>
        </div>

        <div style="margin-bottom:12px;">
          <label style="display:block; font-size:12px; font-weight:600; margin-bottom:4px;">${isId ? 'Judul Acara' : 'Event Title'}</label>
          <input type="text" id="cal-evt-title" placeholder="${isId ? 'Misal: Presentasi Proyek' : 'e.g. Project Presentation'}"
                 style="width:100%; padding:8px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:13px; outline:none;">
        </div>

        <div style="display:grid; grid-template-columns:1fr 1fr; gap:10px; margin-bottom:12px;">
          <div>
            <label style="display:block; font-size:12px; font-weight:600; margin-bottom:4px;">${isId ? 'Tanggal' : 'Date'}</label>
            <input type="date" id="cal-evt-date" value="${this.selectedDateStr}"
                   style="width:100%; padding:8px 10px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:12.5px; outline:none;">
          </div>
          <div>
            <label style="display:block; font-size:12px; font-weight:600; margin-bottom:4px;">${isId ? 'Waktu' : 'Time'}</label>
            <input type="time" id="cal-evt-time" value="09:00"
                   style="width:100%; padding:8px 10px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:12.5px; outline:none;">
          </div>
        </div>

        <div style="margin-bottom:12px;">
          <label style="display:block; font-size:12px; font-weight:600; margin-bottom:4px;">${isId ? 'Kategori' : 'Category'}</label>
          <select id="cal-evt-cat" style="width:100%; padding:8px 10px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:12.5px; outline:none;">
            <option value="work">${isId ? 'Pekerjaan (Biru)' : 'Work (Blue)'}</option>
            <option value="meeting">${isId ? 'Rapat (Ungu)' : 'Meeting (Purple)'}</option>
            <option value="personal">${isId ? 'Pribadi (Hijau)' : 'Personal (Green)'}</option>
            <option value="urgent">${isId ? 'Penting (Merah)' : 'Urgent (Red)'}</option>
          </select>
        </div>

        <div style="margin-bottom:18px;">
          <label style="display:block; font-size:12px; font-weight:600; margin-bottom:4px;">${isId ? 'Catatan (Opsional)' : 'Notes (Optional)'}</label>
          <textarea id="cal-evt-notes" rows="2" placeholder="${isId ? 'Tulis rincian acara...' : 'Add event details...'}"
                    style="width:100%; padding:8px 10px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:12.5px; outline:none; resize:none;"></textarea>
        </div>

        <div style="display:flex; justify-content:flex-end; gap:8px;">
          <button class="mac-btn" onclick="document.getElementById('doku-calendar-modal').style.display='none'">${isId ? 'Batal' : 'Cancel'}</button>
          <button class="mac-btn primary" onclick="DokuCalendar.submitEvent()">💾 ${isId ? 'Simpan Acara' : 'Save Event'}</button>
        </div>
      </div>
    `;
    setTimeout(() => {
      const inp = document.getElementById('cal-evt-title');
      if (inp) inp.focus();
    }, 50);
  },

  submitEvent() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const title = (document.getElementById('cal-evt-title')?.value || '').trim();
    const date = document.getElementById('cal-evt-date')?.value || this.selectedDateStr;
    const time = document.getElementById('cal-evt-time')?.value || '';
    const category = document.getElementById('cal-evt-cat')?.value || 'work';
    const notes = (document.getElementById('cal-evt-notes')?.value || '').trim();

    if (!title) {
      alert(isId ? 'Harap masukkan judul acara.' : 'Please enter an event title.');
      return;
    }

    this.events.push({
      id: `evt_${Date.now()}`,
      date,
      title,
      time,
      category,
      notes
    });
    this.selectedDateStr = date;
    this.saveEvents();

    const modal = document.getElementById('doku-calendar-modal');
    if (modal) modal.style.display = 'none';
    this.render();

    if (window.DokuIsland) {
      DokuIsland.success(isId ? `Acara "${title}" ditambahkan` : `Event "${title}" added`, 'Calendar');
    }
  },

  deleteEvent(id) {
    this.events = this.events.filter(e => e.id !== id);
    this.saveEvents();
    this.render();
  }
};
