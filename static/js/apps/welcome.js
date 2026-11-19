/**
 * DokuHub macOS Welcome Experience
 * Iconic macOS "Hello" / "Selamat Datang" handwriting typography & onboarding flow
 */

(function () {
  'use strict';

  // Welcoming ambient chord synthesizer
  function playWelcomeChord() {
    try {
      const AudioCtx = window.AudioContext || window.webkitAudioContext;
      if (!AudioCtx) return;
      const ctx = new AudioCtx();
      if (ctx.state === 'suspended') ctx.resume();

      const freqs = [261.63, 329.63, 392.00, 523.25]; // C major chord C4, E4, G4, C5
      const now = ctx.currentTime;

      freqs.forEach((freq, idx) => {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = 'sine';
        osc.frequency.setValueAtTime(freq, now + idx * 0.08);

        gain.gain.setValueAtTime(0.0001, now);
        gain.gain.exponentialRampToValueAtTime(0.08 / (idx + 1), now + idx * 0.08 + 0.1);
        gain.gain.exponentialRampToValueAtTime(0.00001, now + 2.5);

        osc.connect(gain);
        gain.connect(ctx.destination);

        osc.start(now + idx * 0.08);
        osc.stop(now + 2.6);
      });

      setTimeout(() => {
        try { ctx.close(); } catch (e) {}
      }, 3000);
    } catch (e) {}
  }

  const GREETINGS = [
    { word: 'hello', lang: 'English' },
    { word: 'halo', lang: 'Bahasa Indonesia' },
    { word: 'bonjour', lang: 'Français' },
    { word: 'hola', lang: 'Español' },
    { word: 'ciao', lang: 'Italiano' },
    { word: 'selamat datang', lang: 'Nusantara' },
    { word: 'welcome', lang: 'BijiOS' }
  ];

  const DokuWelcome = {
    el: null,
    currentIndex: 0,
    cycleTimer: null,
    isAnimating: false,

    init() {
      if (document.getElementById('macos-welcome-overlay')) {
        this.el = document.getElementById('macos-welcome-overlay');
      } else {
        this.el = document.createElement('div');
        this.el.id = 'macos-welcome-overlay';
        this.el.className = 'macos-welcome-overlay hidden';
        document.body.appendChild(this.el);
      }
    },

    /**
     * Check if user is new or has not seen welcome yet, and show if appropriate
     */
    checkAndTrigger(user) {
      if (!user || !user.id) return;
      const key = `dokuhub_welcomed_${user.id}`;
      const seen = localStorage.getItem(key);
      if (!seen) {
        // First login or new user
        setTimeout(() => {
          this.show(user);
        }, 800);
      }
    },

    show(user) {
      if (!this.el) this.init();
      this.isAnimating = true;

      const cur = user || (window.DokuHub && DokuHub.currentUser) || {
        id: 'usr_guest',
        name: 'Pengguna BijiOS',
        email: 'user@bijios.local',
        role: 'Member'
      };

      // Play soft harmonic welcoming chord
      playWelcomeChord();

      this.el.className = 'macos-welcome-overlay active';
      this.el.innerHTML = `
        <div class="welcome-aurora-bg"></div>
        <div class="welcome-center-container">
          <!-- Animated Cursive Script -->
          <div class="welcome-script-stage" id="welcome-script-stage">
            <div class="cursive-text-wrapper" id="cursive-text-wrapper">
              <span class="cursive-word active" id="cursive-word-el">hello</span>
              <div class="cursive-underline-sparkle"></div>
            </div>
            <div class="cursive-lang-tag" id="cursive-lang-tag">English</div>
          </div>

          <!-- Glass Onboarding Card (Revealed after greeting cycle) -->
          <div class="welcome-card-glass" id="welcome-card-glass" style="display:none; opacity:0; transform:translateY(24px) scale(0.96);">
            <div class="welcome-card-header">
              <div class="welcome-apple-logo"></div>
              <h1 class="welcome-headline">Selamat Datang di BijiOS</h1>
              <p class="welcome-subheadline">Ruang kerja kolaboratif bergaya macOS dengan keamanan multi-tenant, cloud storage terisolasi, dan office suite real-time.</p>
            </div>

            <div class="welcome-user-chip">
              <div class="welcome-avatar-wrap" id="welcome-avatar-wrap"></div>
              <div class="welcome-user-info">
                <div class="welcome-user-name">${cur.name || 'Pengguna'}</div>
                <div class="welcome-user-email">${cur.email || ''}</div>
                <div class="welcome-badges-row">
                  <span class="welcome-role-badge">${cur.role || 'Member'}</span>
                  <span class="welcome-tenant-badge">🏢 ${cur.tenant || (window.DokuHub && DokuHub.activeTenant ? DokuHub.activeTenant.name : 'Workspace')}</span>
                </div>
              </div>
            </div>

            <div class="welcome-feature-grid">
              <div class="feature-item">
                <div class="feature-icon">📁</div>
                <div>
                  <div class="feature-title">Penyimpanan Terisolasi</div>
                  <div class="feature-desc">Ruang dokumen pribadi Anda terlindungi enkripsi dan kuota disk khusus.</div>
                </div>
              </div>
              <div class="feature-item">
                <div class="feature-icon">✨</div>
                <div>
                  <div class="feature-title">Liquid Glass & Dynamic Island</div>
                  <div class="feature-desc">Tampilan macOS Golden Gate yang memukau dan notifikasi cerdas.</div>
                </div>
              </div>
              <div class="feature-item">
                <div class="feature-icon">📝</div>
                <div>
                  <div class="feature-title">Word, Grid, & Deck Office</div>
                  <div class="feature-desc">Editor dokumen, spreadsheet, dan presentasi lengkap langsung di desktop.</div>
                </div>
              </div>
            </div>

            <div class="welcome-actions">
              <button class="welcome-btn-primary" id="welcome-btn-start">
                <span>Mulai Menjelajah BijiOS</span>
                <span class="btn-arrow">→</span>
              </button>
            </div>
          </div>

          <!-- Skip / Dismiss button at top right -->
          <button class="welcome-skip-btn" id="welcome-skip-btn">Lewati (Skip) ✕</button>
        </div>
      `;

      // Render Avatar in Welcome Card
      const avatarWrap = document.getElementById('welcome-avatar-wrap');
      if (avatarWrap) {
        const initials = (cur.name || 'User').split(' ').map(n => n[0]).join('').slice(0, 2).toUpperCase();
        const avatar = cur.avatar || initials;
        const color = cur.color || '#007aff';
        if (window.DokuSettings && typeof DokuSettings.renderAvatarHtml === 'function') {
          avatarWrap.innerHTML = DokuSettings.renderAvatarHtml(avatar, color, initials, 56);
        } else {
          avatarWrap.innerHTML = `<div style="width:56px;height:56px;border-radius:50%;background:${color};display:flex;align-items:center;justify-content:center;color:#fff;font-weight:700;font-size:22px;">${initials}</div>`;
        }
      }

      // Hook Events
      const skipBtn = document.getElementById('welcome-skip-btn');
      if (skipBtn) skipBtn.onclick = () => this.dismiss(cur);

      const startBtn = document.getElementById('welcome-btn-start');
      if (startBtn) startBtn.onclick = () => this.dismiss(cur);

      // Start Handwriting Cycle
      this.runGreetingCycle(cur);
    },

    runGreetingCycle(user) {
      this.currentIndex = 0;
      const wordEl = document.getElementById('cursive-word-el');
      const langEl = document.getElementById('cursive-lang-tag');
      const scriptStage = document.getElementById('welcome-script-stage');
      const cardEl = document.getElementById('welcome-card-glass');

      const nextWord = () => {
        if (!this.isAnimating) return;

        if (this.currentIndex >= GREETINGS.length) {
          // Finished cycle: Fade out cursive text and gracefully transition to Glass Card
          if (scriptStage) {
            scriptStage.style.transition = 'all 0.6s cubic-bezier(0.16, 1, 0.3, 1)';
            scriptStage.style.opacity = '0';
            scriptStage.style.transform = 'translateY(-20px) scale(0.95)';
          }

          setTimeout(() => {
            if (scriptStage) scriptStage.style.display = 'none';
            if (cardEl) {
              cardEl.style.display = 'block';
              requestAnimationFrame(() => {
                cardEl.style.transition = 'all 0.8s cubic-bezier(0.16, 1, 0.3, 1)';
                cardEl.style.opacity = '1';
                cardEl.style.transform = 'translateY(0) scale(1)';
              });
            }
          }, 500);
          return;
        }

        const item = GREETINGS[this.currentIndex];
        this.currentIndex++;

        if (wordEl && langEl) {
          wordEl.style.opacity = '0';
          wordEl.style.transform = 'translateY(12px) scale(0.96)';
          langEl.style.opacity = '0';

          setTimeout(() => {
            wordEl.textContent = item.word;
            langEl.textContent = item.lang;

            // Trigger cursive stroke animation
            wordEl.className = 'cursive-word stroke-animating';
            wordEl.style.opacity = '1';
            wordEl.style.transform = 'translateY(0) scale(1)';
            langEl.style.opacity = '0.7';

            this.cycleTimer = setTimeout(nextWord, 1300);
          }, 250);
        }
      };

      // Kick off first word
      nextWord();
    },

    dismiss(user) {
      this.isAnimating = false;
      if (this.cycleTimer) clearTimeout(this.cycleTimer);

      if (user && user.id) {
        localStorage.setItem(`dokuhub_welcomed_${user.id}`, 'true');
      }

      if (this.el) {
        this.el.style.transition = 'opacity 0.5s ease, transform 0.5s ease';
        this.el.style.opacity = '0';
        this.el.style.transform = 'scale(1.03)';
        setTimeout(() => {
          this.el.className = 'macos-welcome-overlay hidden';
          this.el.innerHTML = '';
          this.el.style.opacity = '';
          this.el.style.transform = '';
          this.el.style.display = 'none';

          // Trigger dynamic island welcome alert
          if (window.DokuIsland) {
            const isId = (window.DokuI18n && DokuI18n.lang === 'id');
            DokuIsland.success(isId ? 'Selamat Datang di BijiOS Desktop. Siap bekerja!' : 'Welcome to BijiOS Desktop. Ready to work!', 'BijiOS Desktop');
          }
        }, 500);
      }
    },

    replay() {
      const cur = (window.DokuHub && DokuHub.currentUser) || null;
      if (cur && cur.id) {
        localStorage.removeItem(`dokuhub_welcomed_${cur.id}`);
      }
      this.show(cur);
    }
  };

  window.DokuWelcome = DokuWelcome;

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => DokuWelcome.init());
  } else {
    DokuWelcome.init();
  }
})();
