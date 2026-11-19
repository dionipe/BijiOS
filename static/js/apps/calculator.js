/**
 * DokuCalculator - macOS Sequoia-style Calculator Application
 * Features:
 * - Standard & Scientific modes with instant switcher
 * - Full physical keyboard support (0-9, +, -, *, /, Enter, Backspace, Escape)
 * - Calculation History panel with 1-click recall
 * - Adaptive Light & Dark Mode
 */

window.DokuCalculator = {
  container: null,
  display: '0',
  expression: '',
  resetNext: false,
  scientific: false,
  showHistory: false,
  history: [],
  _keyHandler: null,

  init(containerEl) {
    this.container = containerEl;
    this.display = '0';
    this.expression = '';
    this.resetNext = false;
    this.render();
    this.bindKeyboard();
  },

  bindKeyboard() {
    if (this._keyHandler) {
      window.removeEventListener('keydown', this._keyHandler);
    }
    this._keyHandler = (e) => {
      if (!window.DokuHub || DokuHub.activeAppId !== 'calculator') return;
      if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA')) return;

      if (/^[0-9.]$/.test(e.key)) {
        e.preventDefault();
        this.inputDigit(e.key);
      } else if (['+', '-', '*', '/', '(', ')'].includes(e.key)) {
        e.preventDefault();
        this.inputOperator(e.key);
      } else if (e.key === 'Enter' || e.key === '=') {
        e.preventDefault();
        this.calculate();
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        this.backspace();
      } else if (e.key === 'Escape' || e.key.toLowerCase() === 'c') {
        e.preventDefault();
        this.clearAll();
      } else if (e.key === '%') {
        e.preventDefault();
        this.percent();
      }
    };
    window.addEventListener('keydown', this._keyHandler);
  },

  toggleScientific() {
    this.scientific = !this.scientific;
    const win = document.getElementById('window-calculator');
    if (win) {
      win.classList.add('maximizing-transition');
      win.style.width = this.scientific ? '470px' : '320px';
      setTimeout(() => win.classList.remove('maximizing-transition'), 340);
    }
    this.render();
  },

  toggleHistory() {
    this.showHistory = !this.showHistory;
    this.render();
  },

  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    const sciButtons = [
      { label: '(', action: `DokuCalculator.inputOperator('(')` },
      { label: ')', action: `DokuCalculator.inputOperator(')')` },
      { label: 'x²', action: `DokuCalculator.applyUnary('sq')` },
      { label: '√x', action: `DokuCalculator.applyUnary('sqrt')` },
      { label: 'sin', action: `DokuCalculator.applyUnary('sin')` },
      { label: 'cos', action: `DokuCalculator.applyUnary('cos')` },
      { label: 'tan', action: `DokuCalculator.applyUnary('tan')` },
      { label: 'log', action: `DokuCalculator.applyUnary('log')` },
      { label: 'ln', action: `DokuCalculator.applyUnary('ln')` },
      { label: 'π', action: `DokuCalculator.inputConst('Math.PI', '3.14159265')` },
      { label: 'e', action: `DokuCalculator.inputConst('Math.E', '2.71828182')` },
      { label: '1/x', action: `DokuCalculator.applyUnary('inv')` }
    ];

    this.container.innerHTML = `
      <div style="display:flex; flex-direction:column; height:100%; background:var(--window-bg); color:var(--text-main); font-family:-apple-system, BlinkMacSystemFont, 'SF Pro Display', sans-serif; user-select:none; position:relative;">
        <!-- Top Mode Bar -->
        <div style="display:flex; align-items:center; justify-content:space-between; padding:8px 12px 4px; flex-shrink:0;">
          <button class="mac-btn" onclick="DokuCalculator.toggleHistory()" style="padding:3px 9px; font-size:11px;" title="${isId ? 'Riwayat Hitungan' : 'Calculation History'}">
            🕘 ${isId ? 'Riwayat' : 'History'} (${this.history.length})
          </button>
          <button class="mac-btn ${this.scientific ? 'primary' : ''}" onclick="DokuCalculator.toggleScientific()" style="padding:3px 9px; font-size:11px;">
            ${this.scientific ? (isId ? 'Standar' : 'Basic') : (isId ? 'Ilmiah' : 'Scientific')}
          </button>
        </div>

        <!-- Display Screen -->
        <div style="padding:10px 18px 14px; text-align:right; flex-shrink:0;">
          <div id="calc-expression" style="font-size:13px; color:var(--text-muted); min-height:20px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
            ${this.expression || '&nbsp;'}
          </div>
          <div id="calc-display" style="font-size:40px; font-weight:300; letter-spacing:-1px; line-height:1.15; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; color:var(--text-main);">
            ${this.display}
          </div>
        </div>

        <!-- Keypad Grid -->
        <div style="flex:1; display:flex; gap:8px; padding:0 12px 14px; min-height:0;">
          ${this.scientific ? `
            <div style="display:grid; grid-template-columns:repeat(3, 1fr); gap:8px; width:145px; flex-shrink:0;">
              ${sciButtons.map(b => `
                <button class="calc-btn calc-btn-fn" onclick="${b.action}">${b.label}</button>
              `).join('')}
            </div>
          ` : ''}

          <div style="flex:1; display:grid; grid-template-columns:repeat(4, 1fr); gap:8px;">
            <button class="calc-btn calc-btn-fn" onclick="DokuCalculator.clearAll()">${this.display !== '0' || this.expression ? 'C' : 'AC'}</button>
            <button class="calc-btn calc-btn-fn" onclick="DokuCalculator.toggleSign()">±</button>
            <button class="calc-btn calc-btn-fn" onclick="DokuCalculator.percent()">%</button>
            <button class="calc-btn calc-btn-op" onclick="DokuCalculator.inputOperator('/')">÷</button>

            <button class="calc-btn" onclick="DokuCalculator.inputDigit('7')">7</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('8')">8</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('9')">9</button>
            <button class="calc-btn calc-btn-op" onclick="DokuCalculator.inputOperator('*')">×</button>

            <button class="calc-btn" onclick="DokuCalculator.inputDigit('4')">4</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('5')">5</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('6')">6</button>
            <button class="calc-btn calc-btn-op" onclick="DokuCalculator.inputOperator('-')">−</button>

            <button class="calc-btn" onclick="DokuCalculator.inputDigit('1')">1</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('2')">2</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('3')">3</button>
            <button class="calc-btn calc-btn-op" onclick="DokuCalculator.inputOperator('+')">+</button>

            <button class="calc-btn" onclick="DokuCalculator.backspace()" title="Backspace">⌫</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('0')">0</button>
            <button class="calc-btn" onclick="DokuCalculator.inputDigit('.')">.</button>
            <button class="calc-btn calc-btn-eq" onclick="DokuCalculator.calculate()">=</button>
          </div>
        </div>

        <!-- History Drawer Overlay -->
        ${this.showHistory ? `
          <div style="position:absolute; inset:40px 10px 10px; background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; box-shadow:var(--window-shadow); padding:14px; display:flex; flex-direction:column; z-index:20; animation:macModalPopIn 0.2s cubic-bezier(0.16,1,0.3,1);">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px; padding-bottom:8px; border-bottom:1px solid var(--window-border);">
              <strong style="font-size:13px;">🕘 ${isId ? 'Riwayat Perhitungan' : 'History'}</strong>
              <div style="display:flex; gap:6px;">
                <button class="mac-btn" onclick="DokuCalculator.history=[]; DokuCalculator.render();" style="font-size:10.5px; padding:2px 8px;">${isId ? 'Bersihkan' : 'Clear'}</button>
                <button class="mac-btn" onclick="DokuCalculator.toggleHistory()" style="font-size:10.5px; padding:2px 8px;">✕</button>
              </div>
            </div>
            <div style="flex:1; overflow-y:auto; display:flex; flex-direction:column; gap:6px;">
              ${this.history.length === 0 ? `
                <div style="text-align:center; padding:30px 10px; font-size:12px; color:var(--text-muted);">${isId ? 'Belum ada riwayat.' : 'No calculations yet.'}</div>
              ` : this.history.map(h => `
                <div onclick="DokuCalculator.recallHistory('${h.result}')" style="padding:8px 10px; border-radius:8px; background:var(--sidebar-bg); border:1px solid var(--window-border); cursor:pointer; text-align:right;">
                  <div style="font-size:11px; color:var(--text-muted);">${h.expr}</div>
                  <div style="font-size:16px; font-weight:700; color:var(--text-main);">= ${h.result}</div>
                </div>
              `).join('')}
            </div>
          </div>
        ` : ''}
      </div>
    `;
  },

  updateDisplayOnly() {
    const disp = document.getElementById('calc-display');
    const expr = document.getElementById('calc-expression');
    if (disp) disp.textContent = this.display;
    if (expr) expr.innerHTML = this.expression || '&nbsp;';
  },

  inputDigit(d) {
    if (this.resetNext) {
      this.display = (d === '.') ? '0.' : d;
      this.resetNext = false;
    } else {
      if (d === '.' && this.display.includes('.')) return;
      if (this.display === '0' && d !== '.') {
        this.display = d;
      } else {
        this.display += d;
      }
    }
    this.updateDisplayOnly();
  },

  inputConst(jsName, strVal) {
    this.display = strVal;
    this.resetNext = false;
    this.updateDisplayOnly();
  },

  inputOperator(op) {
    if (op === '(' || op === ')') {
      this.expression += (op === '(' && this.display !== '0' && !this.resetNext ? this.display + '*' : '') + op;
      if (op === ')') {
        this.expression = this.expression.replace(/\)$/, this.display + ')');
      }
      this.resetNext = true;
      this.updateDisplayOnly();
      return;
    }

    if (this.expression && this.resetNext) {
      this.expression = this.expression.replace(/[+\-*/]\s*$/, `${op} `);
    } else {
      this.expression += `${this.display} ${op} `;
    }
    this.resetNext = true;
    this.updateDisplayOnly();
  },

  applyUnary(fn) {
    const val = parseFloat(this.display);
    if (isNaN(val)) return;
    let res = val;
    switch (fn) {
      case 'sq': res = val * val; break;
      case 'sqrt': res = val >= 0 ? Math.sqrt(val) : NaN; break;
      case 'sin': res = Math.sin(val * Math.PI / 180); break;
      case 'cos': res = Math.cos(val * Math.PI / 180); break;
      case 'tan': res = Math.tan(val * Math.PI / 180); break;
      case 'log': res = val > 0 ? Math.log10(val) : NaN; break;
      case 'ln': res = val > 0 ? Math.log(val) : NaN; break;
      case 'inv': res = val !== 0 ? 1 / val : NaN; break;
    }
    this.display = Number.isFinite(res) ? String(parseFloat(res.toFixed(10))) : 'Error';
    this.resetNext = true;
    this.updateDisplayOnly();
  },

  toggleSign() {
    if (this.display === '0' || this.display === 'Error') return;
    this.display = this.display.startsWith('-') ? this.display.slice(1) : '-' + this.display;
    this.updateDisplayOnly();
  },

  percent() {
    const val = parseFloat(this.display);
    if (isNaN(val)) return;
    this.display = String(parseFloat((val / 100).toFixed(10)));
    this.updateDisplayOnly();
  },

  backspace() {
    if (this.resetNext || this.display === 'Error') {
      this.display = '0';
      this.resetNext = false;
    } else {
      this.display = this.display.length > 1 ? this.display.slice(0, -1) : '0';
    }
    this.updateDisplayOnly();
  },

  clearAll() {
    this.display = '0';
    this.expression = '';
    this.resetNext = false;
    this.updateDisplayOnly();
  },

  calculate() {
    try {
      const fullExpr = (this.expression + this.display).trim();
      if (!fullExpr) return;
      // Safe math evaluation (only numbers, operators, parentheses, spaces, dots)
      if (!/^[0-9+\-*/().\s]+$/.test(fullExpr)) {
        this.display = 'Error';
        this.updateDisplayOnly();
        return;
      }
      const result = Function(`"use strict"; return (${fullExpr})`)();
      if (!Number.isFinite(result)) {
        this.display = 'Error';
      } else {
        const formatted = String(parseFloat(result.toFixed(10)));
        const prettyExpr = fullExpr.replace(/\*/g, '×').replace(/\//g, '÷');
        this.history.unshift({ expr: prettyExpr, result: formatted });
        if (this.history.length > 30) this.history.pop();
        this.display = formatted;
        this.expression = '';
        this.resetNext = true;
      }
    } catch {
      this.display = 'Error';
      this.expression = '';
      this.resetNext = true;
    }
    this.render();
  },

  recallHistory(res) {
    this.display = res;
    this.showHistory = false;
    this.resetNext = false;
    this.render();
  }
};
