/**
 * DokuMeet - Sovereign Self-Hosted WebRTC Video Conference & Collaboration
 * Features:
 * - Real WebRTC RTCPeerConnection Mesh with STUN traversal
 * - Full bi-directional Audio & Video streaming (real sound playback through audio elements)
 * - Live Voice Activity Detection (VAD) for both Local & Remote peers (active speaker indicator & waveform)
 * - Screen Presentation Mode with Pinned Stage (75/25 responsive focal view)
 * - In-Call Chat with unique message deduplication (zero looping) & friendly user names
 * - Automatic heartbeat & stale room pruning
 * - Bilingual i18n support (Indonesian & English)
 */

window.DokuMeet = {
  container: null,
  roomCode: null,
  roomTitle: 'BijiMeet Conference',
  peerId: null,
  localStream: null,
  screenStream: null,
  pinnedStream: null,
  pinnedTitle: null,
  pinnedType: null, // 'screen' | 'peer' | null
  isAudioMuted: false,
  isVideoOff: false,
  isScreenSharing: false,
  peers: [],
  peerConnections: {}, // remotePeerId -> RTCPeerConnection
  remoteStreams: {},    // remotePeerId -> MediaStream
  remoteAnalysers: {},  // remotePeerId -> { analyser, buffer }
  pollTimer: null,
  vadTimer: null,
  audioCtx: null,
  localAnalyser: null,
  messages: [],
  renderedMessageIds: new Set(),
  lastSignalId: 0,
  activeRooms: [],
  isSpeaking: false,

  rtcConfig: {
    iceServers: [
      { urls: 'stun:stun.l.google.com:19302' },
      { urls: 'stun:stun1.l.google.com:19302' },
      { urls: 'stun:stun2.l.google.com:19302' }
    ]
  },

  init(containerEl) {
    this.container = containerEl;
    this.peerId = `peer_${Math.random().toString(36).substr(2, 8)}`;
    this.renderLobby();
  },

  getAuthHeaders() {
    const headers = {};
    const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    const curUser = (window.DokuHub && DokuHub.currentUser) || null;
    if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;
    return headers;
  },

  async renderLobby() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    this.cleanupMedia();

    // Fetch genuine active rooms from backend (only rooms with currently connected peers)
    try {
      const res = await fetch('/api/meet/rooms', { headers: this.getAuthHeaders() });
      if (res.ok) {
        this.activeRooms = await res.json();
      }
    } catch {
      this.activeRooms = [];
    }

    this.container.innerHTML = `
      <div style="display:flex; flex-direction:column; height:100%; align-items:center; justify-content:center; background:var(--window-bg); color:var(--text-main); padding:24px; overflow-y:auto; font-family:-apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;">
        <div style="max-width:540px; width:100%; text-align:center;">
          <!-- Logo & Header -->
          <div style="width:72px; height:72px; border-radius:20px; background:linear-gradient(135deg, #10b981, #059669); display:flex; align-items:center; justify-content:center; font-size:36px; margin:0 auto 16px; box-shadow:0 10px 25px rgba(16,185,129,0.35);">
            🎥
          </div>
          <h2 style="font-size:24px; font-weight:700; margin-bottom:8px; color:var(--text-main);">
            ${isId ? 'BijiMeet Video Conference' : 'BijiMeet Video Conference'}
          </h2>
          <p style="font-size:13px; color:var(--text-muted); margin-bottom:24px; line-height:1.5;">
            ${isId
        ? 'Konferensi video & audio WebRTC mandiri. Dilengkapi transmisi suara P2P langsung, berbagi layar yang dapat di-pin, dan tata letak grid multi-peserta.'
        : 'Sovereign WebRTC video & audio conference. Real P2P audio transmission, pinnable screen presentations, and adaptive multi-participant grid.'}
          </p>

          <!-- Create / Join Card -->
          <div style="background:var(--card-bg); border-radius:16px; padding:22px; border:1px solid var(--window-border); margin-bottom:20px; text-align:left; box-shadow:0 10px 30px rgba(0,0,0,0.08);">
            <div style="margin-bottom:14px;">
              <label style="display:block; font-size:12px; font-weight:600; color:var(--text-main); margin-bottom:6px;">
                ${isId ? 'Judul Rapat Baru:' : 'New Meeting Title:'}
              </label>
              <input type="text" id="meet-create-title" 
                     placeholder="${isId ? 'Contoh: Rapat Koordinasi Tim' : 'e.g. Team Standup & Sync'}"
                     value="${isId ? 'Rapat BijiMeet' : 'BijiMeet Conference'}"
                     style="width:100%; padding:10px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:13px; outline:none;">
            </div>

            <div style="display:flex; gap:10px; margin-bottom:18px;">
              <button class="mac-btn primary" style="flex:1; justify-content:center; padding:11px; font-size:13.5px; font-weight:600; border-radius:8px;" onclick="DokuMeet.createInstantMeeting()">
                ➕ ${isId ? 'Mulai Rapat Baru' : 'Start New Meeting'}
              </button>
              <button class="mac-btn" style="padding:11px 14px; font-size:13px;" onclick="DokuMeet.testAudioModal()" title="${isId ? 'Uji Mikrofon & Speaker' : 'Test Mic & Audio'}">
                🎙️ ${isId ? 'Uji Mic' : 'Test Mic'}
              </button>
            </div>

            <div style="text-align:center; font-size:11px; font-weight:700; color:var(--text-muted); letter-spacing:0.5px; margin-bottom:14px;">
              ${isId ? '— ATAU MASUK DENGAN KODE RAPAT —' : '— OR JOIN WITH MEETING CODE —'}
            </div>

            <div style="display:flex; gap:8px;">
              <input type="text" id="meet-join-code" placeholder="${isId ? 'Ketik kode (contoh: biji-8492)' : 'Enter code (e.g. biji-8492)'}" 
                     style="flex:1; padding:9px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main); font-size:13px; outline:none;"
                     onkeydown="if(event.key==='Enter') DokuMeet.joinMeeting(this.value)">
              <button class="mac-btn primary" style="padding:9px 16px; font-weight:600; border-radius:8px;" onclick="DokuMeet.joinMeeting(document.getElementById('meet-join-code').value)">
                ${isId ? 'Gabung' : 'Join'}
              </button>
            </div>
          </div>

          <!-- Active Rooms List -->
          ${this.activeRooms && this.activeRooms.length > 0 ? `
            <div style="background:var(--card-bg); border-radius:14px; padding:16px; border:1px solid var(--window-border); margin-bottom:18px; text-align:left; box-shadow:0 4px 16px rgba(0,0,0,0.06);">
              <div style="font-size:12px; font-weight:700; color:var(--text-muted); text-transform:uppercase; margin-bottom:10px; display:flex; justify-content:space-between; align-items:center;">
                <span style="display:flex; align-items:center; gap:6px;">
                  <span style="width:8px; height:8px; border-radius:50%; background:#10b981; display:inline-block;"></span>
                  ${isId ? 'Rapat Sedang Berlangsung di Tenant' : 'Active Meetings in Tenant'}
                </span>
                <span style="background:rgba(16,185,129,0.15); color:#10b981; padding:2px 8px; border-radius:10px; font-size:11px;">
                  ${this.activeRooms.length} ${isId ? 'Aktif' : 'Active'}
                </span>
              </div>
              <div style="display:flex; flex-direction:column; gap:6px;">
                ${this.activeRooms.map(r => `
                  <div style="display:flex; align-items:center; justify-content:space-between; padding:9px 12px; border-radius:8px; background:var(--sidebar-bg); border:1px solid var(--window-border);">
                    <div>
                      <div style="font-size:13px; font-weight:600; color:var(--text-main);">${r.title}</div>
                      <div style="font-size:11px; color:var(--text-muted); font-family:monospace; margin-top:1px;">
                        ${r.code} • 👥 ${r.active_peers || 1} ${isId ? 'peserta aktif' : 'active members'}
                      </div>
                    </div>
                    <button class="mac-btn primary" onclick="DokuMeet.joinMeeting('${r.code}', '${r.title}')" style="font-size:11.5px; padding:4px 12px;">
                      ${isId ? 'Masuk' : 'Join'}
                    </button>
                  </div>
                `).join('')}
              </div>
            </div>
          ` : `
            <div style="margin-bottom:16px; font-size:12px; color:var(--text-muted); background:var(--sidebar-bg); padding:10px; border-radius:10px; border:1px dashed var(--window-border);">
              🟢 ${isId ? 'Tidak ada ruang rapat aktif. Ruang baru akan otomatis muncul saat Anda memulai.' : 'No active meetings. A room will automatically appear when created.'}
            </div>
          `}

          <div style="display:flex; align-items:center; justify-content:center; gap:8px; font-size:11.5px; color:var(--text-muted);">
            <span style="color:#10b981;">●</span>
            <span>${isId ? 'WebRTC P2P Direct Audio & Video Mesh' : 'WebRTC P2P Direct Audio & Video Mesh'}</span>
          </div>
        </div>
      </div>
    `;
  },

  async createInstantMeeting() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const titleInput = document.getElementById('meet-create-title');
    const title = titleInput ? titleInput.value.trim() : (isId ? 'Rapat BijiMeet' : 'BijiMeet Conference');

    try {
      const res = await fetch('/api/meet/rooms', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...this.getAuthHeaders() },
        body: JSON.stringify({ title })
      });
      const data = await res.json();
      if (data.code) {
        this.joinMeeting(data.code, data.title || title);
      }
    } catch {
      const randomSuffix = Math.floor(1000 + Math.random() * 9000);
      this.joinMeeting(`biji-${randomSuffix}`, title);
    }
  },

  async joinMeeting(code, title = null) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (!code || !code.trim()) {
      alert(isId ? "Masukkan kode ruang rapat yang valid." : "Please enter a valid meeting code.");
      return;
    }
    this.roomCode = code.trim();
    if (title) this.roomTitle = title;
    this.peers = [];
    this.peerConnections = {};
    this.remoteStreams = {};
    this.remoteAnalysers = {};
    this.messages = [];
    this.renderedMessageIds = new Set();
    this.lastSignalId = 0;
    this.pinnedStream = null;
    this.pinnedTitle = null;
    this.pinnedType = null;
    this.isScreenSharing = false;
    this.isAudioMuted = false;
    this.isVideoOff = false;

    // Render Call UI Shell
    this.renderCallView();

    // Acquire Media (Camera & Mic)
    await this.initMediaStreams();

    // Broadcast Join Signal
    await this.sendSignal('join', 'join_room');

    // Notify Dynamic Island
    if (window.DokuIsland) {
      DokuIsland.success(
        isId ? `Tersambung ke ruang ${this.roomCode}` : `Connected to room ${this.roomCode}`,
        'BijiMeet'
      );
    }

    // Start Real-time Signal Polling & Heartbeat
    if (this.pollTimer) clearInterval(this.pollTimer);
    this.pollTimer = setInterval(() => this.pollSignals(), 1500);
  },

  async initMediaStreams() {
    // 1. Try Camera + Microphone
    try {
      this.localStream = await navigator.mediaDevices.getUserMedia({
        video: { width: { ideal: 1280 }, height: { ideal: 720 } },
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }
      });
      this.bindLocalVideo();
      this.setupLocalVoiceActivityDetection(this.localStream);
      return;
    } catch (err) {
      console.warn("Camera + audio not accessible, attempting microphone-only:", err);
    }

    // 2. Fallback: Microphone Only
    try {
      this.localStream = await navigator.mediaDevices.getUserMedia({
        video: false,
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }
      });
      this.isVideoOff = true;
      this.bindLocalVideo();
      this.setupLocalVoiceActivityDetection(this.localStream);
      return;
    } catch (audioErr) {
      console.warn("Microphone not accessible, fallback to avatar simulation:", audioErr);
      this.renderAvatarOnly();
    }
  },

  bindLocalVideo() {
    const localVid = document.getElementById('local-video');
    const avatarEl = document.getElementById('local-avatar-placeholder');
    if (localVid && this.localStream) {
      localVid.srcObject = this.localStream;
      localVid.style.display = this.isVideoOff ? 'none' : 'block';
      if (avatarEl) avatarEl.style.display = this.isVideoOff ? 'flex' : 'none';
    }
  },

  renderAvatarOnly() {
    const localVid = document.getElementById('local-video');
    const avatarEl = document.getElementById('local-avatar-placeholder');
    if (localVid) localVid.style.display = 'none';
    if (avatarEl) avatarEl.style.display = 'flex';
  },

  getOrCreateAudioContext() {
    if (!this.audioCtx || this.audioCtx.state === 'closed') {
      const AudioContextClass = window.AudioContext || window.webkitAudioContext;
      if (AudioContextClass) {
        this.audioCtx = new AudioContextClass();
      }
    }
    if (this.audioCtx && this.audioCtx.state === 'suspended') {
      this.audioCtx.resume();
    }
    return this.audioCtx;
  },

  setupLocalVoiceActivityDetection(stream) {
    try {
      const ctx = this.getOrCreateAudioContext();
      if (!ctx) return;

      const source = ctx.createMediaStreamSource(stream);
      this.localAnalyser = ctx.createAnalyser();
      this.localAnalyser.fftSize = 256;
      this.localAnalyser.smoothingTimeConstant = 0.4;
      source.connect(this.localAnalyser);

      const bufferLength = this.localAnalyser.frequencyBinCount;
      const dataArray = new Uint8Array(bufferLength);

      const loopVAD = () => {
        if (!this.localAnalyser || !this.localStream) return;
        this.localAnalyser.getByteFrequencyData(dataArray);

        let sum = 0;
        for (let i = 0; i < bufferLength; i++) {
          sum += dataArray[i];
        }
        const avg = sum / bufferLength;

        // Threshold for speaking
        const speakingNow = !this.isAudioMuted && avg > 14;
        if (speakingNow !== this.isSpeaking) {
          this.isSpeaking = speakingNow;
          this.updateSpeakingVisuals('local-tile', speakingNow);
        }

        // Also update any connected remote peers' voice visualizers
        this.updateRemoteVADVisuals();

        this.vadTimer = requestAnimationFrame(loopVAD);
      };

      this.vadTimer = requestAnimationFrame(loopVAD);
    } catch (e) {
      console.warn("Local VAD initialization failed:", e);
    }
  },

  updateRemoteVADVisuals() {
    Object.keys(this.remoteAnalysers).forEach(peerId => {
      const item = this.remoteAnalysers[peerId];
      if (item && item.analyser && item.buffer) {
        item.analyser.getByteFrequencyData(item.buffer);
        let sum = 0;
        for (let i = 0; i < item.buffer.length; i++) sum += item.buffer[i];
        const avg = sum / item.buffer.length;
        const speaking = avg > 12;
        this.updateSpeakingVisuals(`remote-tile-${peerId}`, speaking);
      }
    });
  },

  updateSpeakingVisuals(tileId, speaking) {
    const tile = document.getElementById(tileId);
    const wave = document.getElementById(`${tileId}-wave`);
    if (tile) {
      if (speaking) {
        tile.style.border = '2px solid #10b981';
        tile.style.boxShadow = '0 0 20px rgba(16,185,129,0.55)';
      } else {
        tile.style.border = '1px solid var(--window-border)';
        tile.style.boxShadow = '0 8px 24px rgba(0,0,0,0.12)';
      }
    }
    if (wave) {
      wave.style.opacity = speaking ? '1' : '0.35';
    }
  },

  /* ------------------------------------------------------------------------
     WebRTC RTCPeerConnection Mesh
     ------------------------------------------------------------------------ */
  getOrCreatePeerConnection(remotePeerId, isInitiator = false) {
    if (this.peerConnections[remotePeerId]) {
      return this.peerConnections[remotePeerId];
    }

    const pc = new RTCPeerConnection(this.rtcConfig);
    this.peerConnections[remotePeerId] = pc;

    // Add local tracks (Audio & Video)
    if (this.localStream) {
      this.localStream.getTracks().forEach(track => {
        pc.addTrack(track, this.localStream);
      });
    }

    // ICE Candidate handler
    pc.onicecandidate = (event) => {
      if (event.candidate) {
        this.sendSignal('candidate', JSON.stringify(event.candidate), remotePeerId);
      }
    };

    // Track handler: receive incoming remote audio / video
    pc.ontrack = (event) => {
      const stream = event.streams[0] || new MediaStream([event.track]);
      this.remoteStreams[remotePeerId] = stream;

      // 1. Play real sound through dedicated <audio> element
      this.attachRemoteAudio(remotePeerId, stream);

      // 2. Setup Remote Voice Activity Detection
      this.setupRemoteAudioVAD(remotePeerId, stream);

      // 3. Attach video if present
      this.attachRemoteVideo(remotePeerId, stream);
    };

    pc.onconnectionstatechange = () => {
      if (pc.connectionState === 'disconnected' || pc.connectionState === 'failed' || pc.connectionState === 'closed') {
        this.removePeer(remotePeerId);
      }
    };

    // If initiator, create and send Offer
    if (isInitiator) {
      pc.createOffer({ offerToReceiveAudio: true, offerToReceiveVideo: true })
        .then(offer => pc.setLocalDescription(offer))
        .then(() => {
          this.sendSignal('offer', JSON.stringify(pc.localDescription), remotePeerId);
        })
        .catch(err => console.warn(`Error creating offer to ${remotePeerId}:`, err));
    }

    return pc;
  },

  attachRemoteAudio(peerId, stream) {
    let audioEl = document.getElementById(`audio-sink-${peerId}`);
    if (!audioEl) {
      audioEl = document.createElement('audio');
      audioEl.id = `audio-sink-${peerId}`;
      audioEl.autoplay = true;
      audioEl.playsInline = true;
      audioEl.style.display = 'none';
      document.body.appendChild(audioEl);
    }
    audioEl.srcObject = stream;
    audioEl.muted = false;
    audioEl.volume = 1.0;
    audioEl.play().catch(e => {
      console.warn(`Autoplay audio for ${peerId} blocked, will resume on click:`, e);
      const resumeAudio = () => {
        audioEl.play().catch(() => { });
        document.removeEventListener('click', resumeAudio);
      };
      document.addEventListener('click', resumeAudio, { once: true });
    });
  },

  setupRemoteAudioVAD(peerId, stream) {
    try {
      const ctx = this.getOrCreateAudioContext();
      if (!ctx) return;

      const source = ctx.createMediaStreamSource(stream);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.4;
      source.connect(analyser);

      const buffer = new Uint8Array(analyser.frequencyBinCount);
      this.remoteAnalysers[peerId] = { analyser, buffer };
    } catch (e) {
      console.warn(`Remote VAD for ${peerId} failed:`, e);
    }
  },

  attachRemoteVideo(peerId, stream) {
    const videoEl = document.getElementById(`remote-video-${peerId}`);
    const avatarEl = document.getElementById(`remote-avatar-${peerId}`);
    const hasVideo = stream.getVideoTracks().length > 0;

    if (videoEl) {
      videoEl.srcObject = stream;
      videoEl.style.display = hasVideo ? 'block' : 'none';
      videoEl.play().catch(() => { });
    }
    if (avatarEl) {
      avatarEl.style.display = hasVideo ? 'none' : 'flex';
    }
  },

  async handleIncomingSignal(sig) {
    const fromPeer = sig.from_peer;
    if (!fromPeer || fromPeer === this.peerId) return;

    if (sig.type === 'offer') {
      try {
        const pc = this.getOrCreatePeerConnection(fromPeer, false);
        const desc = new RTCSessionDescription(JSON.parse(sig.data));
        await pc.setRemoteDescription(desc);
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        await this.sendSignal('answer', JSON.stringify(answer), fromPeer);
      } catch (e) {
        console.warn(`Error handling offer from ${fromPeer}:`, e);
      }
    } else if (sig.type === 'answer') {
      try {
        const pc = this.peerConnections[fromPeer];
        if (pc && pc.signalingState !== 'stable') {
          const desc = new RTCSessionDescription(JSON.parse(sig.data));
          await pc.setRemoteDescription(desc);
        }
      } catch (e) {
        console.warn(`Error handling answer from ${fromPeer}:`, e);
      }
    } else if (sig.type === 'candidate') {
      try {
        const pc = this.peerConnections[fromPeer];
        if (pc && sig.data) {
          const candidate = new RTCIceCandidate(JSON.parse(sig.data));
          await pc.addIceCandidate(candidate);
        }
      } catch (e) {
        console.warn(`Error handling candidate from ${fromPeer}:`, e);
      }
    } else if (sig.type === 'join') {
      // New peer joined: if our peerId is lexicographically greater, initiate the connection
      if (this.peerId > fromPeer) {
        this.getOrCreatePeerConnection(fromPeer, true);
      }
    } else if (sig.type === 'leave') {
      this.removePeer(fromPeer);
    }
  },

  removePeer(peerId) {
    if (this.peerConnections[peerId]) {
      try { this.peerConnections[peerId].close(); } catch { }
      delete this.peerConnections[peerId];
    }
    delete this.remoteStreams[peerId];
    delete this.remoteAnalysers[peerId];

    const audioSink = document.getElementById(`audio-sink-${peerId}`);
    if (audioSink) audioSink.remove();

    this.peers = this.peers.filter(p => p.peer_id !== peerId);
    this.updateRemotePeersGrid(this.peers);
  },

  /* ------------------------------------------------------------------------
     Call Shell UI Render
     ------------------------------------------------------------------------ */
  renderCallView() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curUserName = (window.DokuHub && DokuHub.currentUser && DokuHub.currentUser.name)
      ? DokuHub.currentUser.name
      : 'User';
    const initial = curUserName.charAt(0).toUpperCase();

    this.container.innerHTML = `
      <div class="meet-layout" style="display:flex; flex-direction:column; height:100%; background:var(--sidebar-bg); color:var(--text-main); font-family:-apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;">
        <!-- Top Info Header -->
        <div style="height:48px; background:var(--window-header-bg); border-bottom:1px solid var(--window-border); display:flex; align-items:center; justify-content:space-between; padding:0 16px; flex-shrink:0;">
          <div style="display:flex; align-items:center; gap:10px;">
            <span style="display:inline-flex; align-items:center; gap:5px; background:rgba(239,68,68,0.16); color:#ef4444; padding:3px 8px; border-radius:12px; font-size:11px; font-weight:700;">
              <span style="width:6px; height:6px; border-radius:50%; background:#ef4444; animation:pulse 1.5s infinite;"></span>
              LIVE
            </span>
            <strong style="font-size:13.5px; color:var(--text-main);">${this.roomTitle}</strong>
            <span style="font-size:11px; color:var(--text-muted); font-family:monospace; background:rgba(125,125,125,0.12); padding:2px 8px; border-radius:6px;">
              ${this.roomCode}
            </span>
            <button class="mac-btn" style="padding:3px 10px; font-size:11px;" onclick="DokuMeet.copyRoomLink()">
              📋 ${isId ? 'Salin Undangan' : 'Copy Invite'}
            </button>
          </div>

          <div style="display:flex; align-items:center; gap:14px; font-size:12px; color:var(--text-muted);">
            <div id="meet-peer-count" style="display:flex; align-items:center; gap:6px;">
              <span>👥</span>
              <span>1 ${isId ? 'Peserta Terhubung' : 'Participant Connected'}</span>
            </div>
            <div style="display:flex; align-items:center; gap:4px; color:#10b981; font-size:11px;">
              <span>●</span>
              <span>WebRTC Encrypted P2P</span>
            </div>
          </div>
        </div>

        <!-- Main Body: Pinned Stage + Gallery or Video Grid -->
        <div id="meet-main-stage-container" style="flex:1; display:flex; overflow:hidden; position:relative;">
          <!-- 1. Pinned Presentation Stage (Focal 75% View) -->
          <div id="meet-pinned-wrapper" style="display:none; flex:1; flex-direction:column; padding:12px; min-width:0; background:var(--window-bg);">
            <div style="height:32px; display:flex; align-items:center; justify-content:space-between; padding:0 8px; margin-bottom:8px; font-size:12px;">
              <span id="meet-pinned-title" style="font-weight:700; color:var(--accent-color); display:flex; align-items:center; gap:6px;">
                🖥️ ${isId ? 'Presentasi Berbagi Layar Aktif' : 'Screen Presentation Active'}
              </span>
              <button class="mac-btn" onclick="DokuMeet.unpinStage()" style="padding:3px 10px; font-size:11px;">
                ✕ ${isId ? 'Lepas Pin (Kembali ke Grid)' : 'Unpin Stage (Back to Grid)'}
              </button>
            </div>
            <div style="flex:1; display:flex; align-items:center; justify-content:center; background:#090d16; border-radius:12px; overflow:hidden; border:1px solid var(--window-border);">
              <video id="meet-pinned-video" autoplay playsinline style="width:100%; height:100%; object-fit:contain;"></video>
            </div>
          </div>

          <!-- 2. Responsive Video Tiles Area -->
          <div id="meet-gallery-wrapper" style="flex:1; padding:14px; display:flex; align-items:center; justify-content:center; overflow-y:auto; transition:all 0.3s;">
            <div id="meet-video-grid" style="width:100%; height:100%; display:grid; grid-template-columns:1fr; gap:14px; align-items:center; justify-items:center; max-width:1100px;">
              
              <!-- Local User Tile -->
              <div class="video-tile" id="local-tile" style="position:relative; width:100%; height:100%; max-height:560px; aspect-ratio:16/9; background:var(--card-bg); border-radius:14px; overflow:hidden; border:1px solid var(--window-border); display:flex; align-items:center; justify-content:center; box-shadow:0 8px 24px rgba(0,0,0,0.12); transition:border 0.2s, box-shadow 0.2s;">
                <video id="local-video" autoplay playsinline muted style="width:100%; height:100%; object-fit:cover;"></video>
                
                <!-- Fallback Avatar -->
                <div id="local-avatar-placeholder" style="display:none; width:100%; height:100%; align-items:center; justify-content:center; flex-direction:column; background:var(--card-bg);">
                  <div style="width:84px; height:84px; border-radius:50%; background:linear-gradient(135deg, #10b981, #059669); display:flex; align-items:center; justify-content:center; font-size:36px; font-weight:700; color:#fff; box-shadow:0 8px 24px rgba(16,185,129,0.35);">
                    ${initial}
                  </div>
                  <div style="font-size:14px; font-weight:600; color:var(--text-main); margin-top:12px;">${curUserName}</div>
                </div>

                <!-- Tile Name & Audio Wave Badge -->
                <div class="peer-name-tag" style="position:absolute; bottom:12px; left:12px; background:rgba(0,0,0,0.65); color:#fff; backdrop-filter:blur(10px); padding:4px 10px; border-radius:8px; font-size:11.5px; font-weight:600; display:flex; align-items:center; gap:8px;">
                  <span id="local-mic-indicator">🎙️</span>
                  <span>${curUserName} (${isId ? 'Anda' : 'You'})</span>
                  <span id="local-tile-wave" style="display:inline-flex; gap:2px; align-items:center; opacity:0.35;">
                    <span style="width:3px; height:8px; background:#10b981; border-radius:2px;"></span>
                    <span style="width:3px; height:12px; background:#10b981; border-radius:2px;"></span>
                    <span style="width:3px; height:6px; background:#10b981; border-radius:2px;"></span>
                  </span>
                </div>

                <!-- Pin Button -->
                <button class="mac-btn" onclick="DokuMeet.pinTile('local', '${curUserName}')" style="position:absolute; top:12px; right:12px; padding:3px 8px; font-size:11px; background:rgba(0,0,0,0.55); color:#fff; border:none;" title="${isId ? 'Pin ke Layar Utama' : 'Pin to Main Stage'}">
                  📌
                </button>
              </div>

              <!-- Remote Peer Tiles Injection Container -->
              <div id="remote-peers-container" style="display:contents;"></div>
            </div>
          </div>

          <!-- 3. In-Call Chat Sidebar -->
          <div id="meet-chat-sidebar" style="width:300px; background:var(--card-bg); border-left:1px solid var(--window-border); display:none; flex-direction:column; flex-shrink:0;">
            <div style="padding:14px 16px; border-bottom:1px solid var(--window-border); font-weight:700; font-size:13.5px; color:var(--text-main); display:flex; justify-content:space-between; align-items:center;">
              <span style="display:flex; align-items:center; gap:6px;">
                <span>💬</span> ${isId ? 'Obrolan Dalam Rapat' : 'In-Call Chat'}
              </span>
              <button class="traffic-btn btn-close" onclick="DokuMeet.toggleChat()"></button>
            </div>
            <div id="meet-chat-messages" style="flex:1; padding:14px; overflow-y:auto; display:flex; flex-direction:column; gap:10px; font-size:12px;"></div>
            <div style="padding:12px; border-top:1px solid var(--window-border); display:flex; gap:6px; background:var(--window-header-bg);">
              <input type="text" id="meet-chat-input" placeholder="${isId ? 'Ketik pesan...' : 'Type a message...'}" 
                     style="flex:1; padding:8px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:12px; outline:none;" 
                     onkeydown="if(event.key==='Enter') DokuMeet.sendChatMessage()">
              <button class="mac-btn primary" onclick="DokuMeet.sendChatMessage()" style="padding:8px 14px; font-weight:600; border-radius:8px;">
                ${isId ? 'Kirim' : 'Send'}
              </button>
            </div>
          </div>
        </div>

        <!-- Controls Bar -->
        <div class="meet-controls-bar" style="height:64px; background:var(--window-header-bg); border-top:1px solid var(--window-border); display:flex; align-items:center; justify-content:center; gap:14px; flex-shrink:0;">
          <button class="meet-ctrl-btn" id="btn-toggle-mic" title="${isId ? 'Nyalakan/Matikan Mikrofon' : 'Mute/Unmute Mic'}" onclick="DokuMeet.toggleAudio()"
                  style="width:44px; height:44px; border-radius:50%; background:var(--card-bg); border:1px solid var(--window-border); color:var(--text-main); font-size:18px; cursor:pointer; display:flex; align-items:center; justify-content:center;">
            🎙️
          </button>
          <button class="meet-ctrl-btn" id="btn-toggle-cam" title="${isId ? 'Nyalakan/Matikan Kamera' : 'Turn On/Off Camera'}" onclick="DokuMeet.toggleVideo()"
                  style="width:44px; height:44px; border-radius:50%; background:var(--card-bg); border:1px solid var(--window-border); color:var(--text-main); font-size:18px; cursor:pointer; display:flex; align-items:center; justify-content:center;">
            📹
          </button>
          <button class="meet-ctrl-btn" id="btn-toggle-screen" title="${isId ? 'Bagikan Layar & Pin Presentasi' : 'Share Screen & Pin Presentation'}" onclick="DokuMeet.toggleScreenShare()"
                  style="width:44px; height:44px; border-radius:50%; background:var(--card-bg); border:1px solid var(--window-border); color:var(--text-main); font-size:18px; cursor:pointer; display:flex; align-items:center; justify-content:center;">
            🖥️
          </button>
          <button class="meet-ctrl-btn" id="btn-test-audio" title="${isId ? 'Uji Suara & Mic' : 'Test Audio & Mic'}" onclick="DokuMeet.testAudioModal()"
                  style="width:44px; height:44px; border-radius:50%; background:var(--card-bg); border:1px solid var(--window-border); color:var(--text-main); font-size:18px; cursor:pointer; display:flex; align-items:center; justify-content:center;">
            🔊
          </button>
          <button class="meet-ctrl-btn" title="${isId ? 'Buka/Tutup Obrolan' : 'Toggle Chat'}" onclick="DokuMeet.toggleChat()"
                  style="width:44px; height:44px; border-radius:50%; background:var(--card-bg); border:1px solid var(--window-border); color:var(--text-main); font-size:18px; cursor:pointer; display:flex; align-items:center; justify-content:center;">
            💬
          </button>
          <button class="meet-ctrl-btn danger" title="${isId ? 'Tinggalkan Rapat' : 'Leave Meeting'}" onclick="DokuMeet.leaveMeeting()"
                  style="width:44px; height:44px; border-radius:50%; background:#ef4444; border:none; color:#fff; font-size:18px; cursor:pointer; display:flex; align-items:center; justify-content:center;">
            📞
          </button>
        </div>
      </div>
    `;

    this.recalculateGridLayout();
  },

  recalculateGridLayout() {
    const grid = document.getElementById('meet-video-grid');
    const galleryWrapper = document.getElementById('meet-gallery-wrapper');
    if (!grid) return;

    const remotePeers = (this.peers || []).filter(p => p.peer_id !== this.peerId);
    const totalCount = remotePeers.length + 1;

    // If presentation is pinned:
    if (this.pinnedType) {
      if (galleryWrapper) {
        galleryWrapper.style.flex = '0 0 280px';
        galleryWrapper.style.maxWidth = '300px';
        galleryWrapper.style.borderLeft = '1px solid var(--window-border)';
        galleryWrapper.style.background = 'var(--sidebar-bg)';
        galleryWrapper.style.padding = '8px';
      }
      grid.style.gridTemplateColumns = '1fr';
      grid.style.maxHeight = '100%';
      return;
    }

    // Default Grid Layout without pinned stage
    if (galleryWrapper) {
      galleryWrapper.style.flex = '1';
      galleryWrapper.style.maxWidth = 'none';
      galleryWrapper.style.borderLeft = 'none';
      galleryWrapper.style.background = 'transparent';
      galleryWrapper.style.padding = '14px';
    }

    if (totalCount === 1) {
      grid.style.gridTemplateColumns = '1fr';
      grid.style.maxWidth = '840px';
    } else if (totalCount === 2) {
      grid.style.gridTemplateColumns = '1fr 1fr';
      grid.style.maxWidth = '1100px';
    } else if (totalCount <= 4) {
      grid.style.gridTemplateColumns = '1fr 1fr';
      grid.style.maxWidth = '1100px';
    } else {
      grid.style.gridTemplateColumns = 'repeat(auto-fit, minmax(280px, 1fr))';
      grid.style.maxWidth = '100%';
    }
  },

  updateRemotePeersGrid(peersList) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const container = document.getElementById('remote-peers-container');
    if (!container) return;

    this.peers = peersList || [];
    const remotePeers = this.peers.filter(p => p.peer_id !== this.peerId);

    // Update count in header
    const countEl = document.getElementById('meet-peer-count');
    if (countEl) {
      const total = remotePeers.length + 1;
      countEl.innerHTML = `<span>👥</span> <span>${total} ${isId ? 'Peserta Terhubung' : 'Participants Connected'}</span>`;
    }

    if (remotePeers.length === 0) {
      container.innerHTML = '';
      this.recalculateGridLayout();
      return;
    }

    const colors = ['#3b82f6', '#8b5cf6', '#ec4899', '#f59e0b', '#06b6d4', '#10b981'];

    container.innerHTML = remotePeers.map((p, idx) => {
      const name = p.user_name || 'Participant';
      const initial = name ? name[0].toUpperCase() : 'P';
      const color = colors[idx % colors.length];

      return `
        <div class="video-tile" id="remote-tile-${p.peer_id}" 
             style="position:relative; width:100%; height:100%; max-height:560px; aspect-ratio:16/9; background:var(--card-bg); border-radius:14px; overflow:hidden; border:1px solid var(--window-border); display:flex; align-items:center; justify-content:center; box-shadow:0 8px 24px rgba(0,0,0,0.12); transition:border 0.2s, box-shadow 0.2s;">
          
          <!-- Remote Video Element -->
          <video id="remote-video-${p.peer_id}" autoplay playsinline style="width:100%; height:100%; object-fit:cover; display:none;"></video>

          <!-- Remote Avatar Fallback -->
          <div id="remote-avatar-${p.peer_id}" style="text-align:center; padding:20px; display:flex; flex-direction:column; align-items:center;">
            <div style="width:72px; height:72px; border-radius:50%; background:${color}; display:flex; align-items:center; justify-content:center; font-size:30px; font-weight:700; color:#fff; margin:0 auto 12px; box-shadow:0 6px 18px rgba(0,0,0,0.22);">
              ${initial}
            </div>
            <div style="font-weight:700; font-size:15px; color:var(--text-main);">${name}</div>
            <div style="font-size:11.5px; color:#10b981; margin-top:4px;">● ${isId ? 'Terhubung (Audio Aktif)' : 'Connected (Audio Active)'}</div>
          </div>

          <!-- Name & Mic Audio Wave Tag -->
          <div class="peer-name-tag" style="position:absolute; bottom:12px; left:12px; background:rgba(0,0,0,0.65); color:#fff; backdrop-filter:blur(10px); padding:4px 10px; border-radius:8px; font-size:11.5px; font-weight:600; display:flex; align-items:center; gap:8px;">
            <span>🎙️</span>
            <span>${name}</span>
            <span id="remote-tile-${p.peer_id}-wave" style="display:inline-flex; gap:2px; align-items:center; opacity:0.35;">
              <span style="width:3px; height:8px; background:#10b981; border-radius:2px;"></span>
              <span style="width:3px; height:12px; background:#10b981; border-radius:2px;"></span>
              <span style="width:3px; height:6px; background:#10b981; border-radius:2px;"></span>
            </span>
          </div>

          <!-- Pin Button -->
          <button class="mac-btn" onclick="DokuMeet.pinTile('${p.peer_id}', '${name}')" style="position:absolute; top:12px; right:12px; padding:3px 8px; font-size:11px; background:rgba(0,0,0,0.55); color:#fff; border:none;" title="${isId ? 'Pin ke Layar Utama' : 'Pin to Main Stage'}">
            📌
          </button>
        </div>
      `;
    }).join('');

    // Re-bind remote streams to newly rendered DOM elements
    remotePeers.forEach(p => {
      const stream = this.remoteStreams[p.peer_id];
      if (stream) {
        this.attachRemoteAudio(p.peer_id, stream);
        this.attachRemoteVideo(p.peer_id, stream);
      } else {
        // If no peer connection exists yet, create one
        this.getOrCreatePeerConnection(p.peer_id, this.peerId > p.peer_id);
      }
    });

    this.recalculateGridLayout();
  },

  pinTile(peerKey, name) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const pinnedWrapper = document.getElementById('meet-pinned-wrapper');
    const pinnedVideo = document.getElementById('meet-pinned-video');
    const pinnedTitle = document.getElementById('meet-pinned-title');

    if (!pinnedWrapper || !pinnedVideo) return;

    this.pinnedType = 'peer';
    this.pinnedTitle = name;

    pinnedWrapper.style.display = 'flex';
    if (pinnedTitle) {
      pinnedTitle.innerHTML = `📌 ${isId ? 'Dipin: ' : 'Pinned: '} <strong>${name}</strong>`;
    }

    if (peerKey === 'local' && this.localStream) {
      pinnedVideo.srcObject = this.localStream;
      pinnedVideo.play().catch(() => { });
    } else if (this.remoteStreams[peerKey]) {
      pinnedVideo.srcObject = this.remoteStreams[peerKey];
      pinnedVideo.play().catch(() => { });
    }

    this.recalculateGridLayout();
  },

  unpinStage() {
    const pinnedWrapper = document.getElementById('meet-pinned-wrapper');
    const pinnedVideo = document.getElementById('meet-pinned-video');
    if (pinnedWrapper) pinnedWrapper.style.display = 'none';
    if (pinnedVideo) pinnedVideo.srcObject = null;

    if (this.isScreenSharing) {
      this.stopScreenShare();
    }

    this.pinnedType = null;
    this.pinnedTitle = null;
    this.recalculateGridLayout();
  },

  toggleAudio() {
    this.isAudioMuted = !this.isAudioMuted;
    if (this.localStream) {
      this.localStream.getAudioTracks().forEach(t => t.enabled = !this.isAudioMuted);
    }
    const btn = document.getElementById('btn-toggle-mic');
    const indicator = document.getElementById('local-mic-indicator');
    if (btn) {
      btn.style.background = this.isAudioMuted ? '#ef4444' : 'var(--card-bg)';
      btn.style.color = this.isAudioMuted ? '#fff' : 'var(--text-main)';
      btn.textContent = this.isAudioMuted ? '🔇' : '🎙️';
    }
    if (indicator) {
      indicator.textContent = this.isAudioMuted ? '🔇' : '🎙️';
    }
    this.updateSpeakingVisuals('local-tile', false);
  },

  toggleVideo() {
    this.isVideoOff = !this.isVideoOff;
    if (this.localStream) {
      this.localStream.getVideoTracks().forEach(t => t.enabled = !this.isVideoOff);
    }
    const btn = document.getElementById('btn-toggle-cam');
    if (btn) {
      btn.style.background = this.isVideoOff ? '#ef4444' : 'var(--card-bg)';
      btn.style.color = this.isVideoOff ? '#fff' : 'var(--text-main)';
      btn.textContent = this.isVideoOff ? '🚫' : '📹';
    }
    this.bindLocalVideo();
  },

  async toggleScreenShare() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (!this.isScreenSharing) {
      try {
        this.screenStream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: true });
        this.isScreenSharing = true;

        const btn = document.getElementById('btn-toggle-screen');
        if (btn) btn.style.background = '#10b981';

        // Replace video tracks on all active peer connections
        const screenTrack = this.screenStream.getVideoTracks()[0];
        Object.values(this.peerConnections).forEach(pc => {
          const sender = pc.getSenders().find(s => s.track && s.track.kind === 'video');
          if (sender) {
            sender.replaceTrack(screenTrack);
          }
        });

        // Pin Screen Stream immediately to the focal Pinned Stage
        const pinnedWrapper = document.getElementById('meet-pinned-wrapper');
        const pinnedVideo = document.getElementById('meet-pinned-video');
        const pinnedTitle = document.getElementById('meet-pinned-title');

        if (pinnedWrapper && pinnedVideo) {
          pinnedWrapper.style.display = 'flex';
          pinnedVideo.srcObject = this.screenStream;
          pinnedVideo.play().catch(() => { });
          this.pinnedType = 'screen';
          if (pinnedTitle) {
            pinnedTitle.innerHTML = `🖥️ ${isId ? 'Presentasi Berbagi Layar Anda (Dipin)' : 'Your Screen Presentation (Pinned)'}`;
          }
        }

        this.recalculateGridLayout();

        screenTrack.onended = () => {
          this.stopScreenShare();
        };

        if (window.DokuIsland) {
          DokuIsland.success(
            isId ? 'Berbagi layar dimulai & dipin ke layar utama' : 'Screen sharing started & pinned to stage',
            'BijiMeet'
          );
        }
      } catch (err) {
        console.warn("Screen share cancelled or failed:", err);
      }
    } else {
      this.stopScreenShare();
    }
  },

  stopScreenShare() {
    if (this.screenStream) {
      this.screenStream.getTracks().forEach(t => t.stop());
      this.screenStream = null;
    }
    this.isScreenSharing = false;
    const btn = document.getElementById('btn-toggle-screen');
    if (btn) {
      btn.style.background = 'var(--card-bg)';
      btn.style.color = 'var(--text-main)';
    }

    // Restore original camera track on all peer connections
    if (this.localStream) {
      const cameraTrack = this.localStream.getVideoTracks()[0];
      if (cameraTrack) {
        Object.values(this.peerConnections).forEach(pc => {
          const sender = pc.getSenders().find(s => s.track && s.track.kind === 'video');
          if (sender) {
            sender.replaceTrack(cameraTrack);
          }
        });
      }
    }

    if (this.pinnedType === 'screen') {
      this.unpinStage();
    }
  },

  testAudioModal() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    let modal = document.getElementById('meet-audio-test-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'meet-audio-test-modal';
      modal.style.position = 'fixed';
      modal.style.inset = '0';
      modal.style.background = 'rgba(0,0,0,0.6)';
      modal.style.backdropFilter = 'blur(15px)';
      modal.style.zIndex = '999999';
      modal.style.display = 'none';
      modal.style.alignItems = 'center';
      modal.style.justifyContent = 'center';
      document.body.appendChild(modal);
    }

    modal.style.display = 'flex';
    modal.innerHTML = `
      <div style="width:480px; max-width:92vw; background:var(--window-bg); border:1px solid var(--window-border); border-radius:18px; box-shadow:var(--window-shadow); padding:24px; color:var(--text-main); animation:macModalPopIn 0.24s cubic-bezier(0.16, 1, 0.3, 1);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <h3 style="font-size:16px; font-weight:700; margin:0; display:flex; align-items:center; gap:8px;">
            <span>🎙️</span> ${isId ? 'Uji Perangkat Mikrofon & Suara' : 'Test Mic & Audio Hardware'}
          </h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('meet-audio-test-modal').style.display='none'"></button>
        </div>

        <p style="font-size:12.5px; color:var(--text-muted); margin-bottom:16px; line-height:1.5;">
          ${isId
        ? 'Bicaralah ke mikrofon Anda untuk menguji input suara. Batang indikator di bawah akan berayun hijau jika suara terdeteksi.'
        : 'Speak into your microphone to test voice input. The level meter below will fluctuate in green as voice is captured.'}
        </p>

        <!-- Live Volume Meter Bar -->
        <div style="margin-bottom:20px;">
          <div style="display:flex; justify-content:space-between; font-size:12px; margin-bottom:6px;">
            <span>${isId ? 'Tingkat Volume Suara Mic:' : 'Microphone Input Level:'}</span>
            <span id="meet-test-vol-pct" style="font-weight:700; color:#10b981;">0%</span>
          </div>
          <div style="width:100%; height:12px; background:var(--sidebar-bg); border-radius:6px; overflow:hidden; border:1px solid var(--window-border);">
            <div id="meet-test-vol-bar" style="width:0%; height:100%; background:linear-gradient(90deg, #10b981, #3b82f6); border-radius:6px; transition:width 0.08s;"></div>
          </div>
        </div>

        <!-- Speaker Tone Test -->
        <div style="margin-bottom:22px; padding:14px; background:var(--sidebar-bg); border-radius:10px; border:1px solid var(--window-border); display:flex; justify-content:space-between; align-items:center;">
          <div>
            <div style="font-size:13px; font-weight:600;">${isId ? 'Uji Speaker / Headset' : 'Test Speaker / Headset'}</div>
            <div style="font-size:11.5px; color:var(--text-muted);">${isId ? 'Putar nada uji untuk memeriksa suara keluar' : 'Play test chime to verify output audio'}</div>
          </div>
          <button class="mac-btn primary" onclick="DokuMeet.playTestChime()" style="font-size:11.5px; padding:6px 14px;">
            🔔 ${isId ? 'Bunyikan' : 'Play Chime'}
          </button>
        </div>

        <div style="display:flex; justify-content:flex-end;">
          <button class="mac-btn primary" onclick="document.getElementById('meet-audio-test-modal').style.display='none'">
            ${isId ? 'Selesai' : 'Done'}
          </button>
        </div>
      </div>
    `;

    // Hook meter update
    const meterBar = document.getElementById('meet-test-vol-bar');
    const meterPct = document.getElementById('meet-test-vol-pct');
    if (this.localAnalyser) {
      const buffer = new Uint8Array(this.localAnalyser.frequencyBinCount);
      const testLoop = () => {
        if (!modal || modal.style.display === 'none') return;
        this.localAnalyser.getByteFrequencyData(buffer);
        let sum = 0;
        for (let i = 0; i < buffer.length; i++) sum += buffer[i];
        const val = Math.min(100, Math.round((sum / buffer.length) * 1.5));
        if (meterBar) meterBar.style.width = `${val}%`;
        if (meterPct) meterPct.textContent = `${val}%`;
        requestAnimationFrame(testLoop);
      };
      requestAnimationFrame(testLoop);
    }
  },

  playTestChime() {
    try {
      const ctx = this.getOrCreateAudioContext();
      if (!ctx) return;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(587.33, ctx.currentTime); // D5
      osc.frequency.setValueAtTime(880, ctx.currentTime + 0.15); // A5
      gain.gain.setValueAtTime(0.15, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.5);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.5);
    } catch (e) {
      console.warn("Chime error:", e);
    }
  },

  toggleChat() {
    const sidebar = document.getElementById('meet-chat-sidebar');
    if (sidebar) {
      sidebar.style.display = sidebar.style.display === 'flex' ? 'none' : 'flex';
    }
  },

  async sendChatMessage() {
    const input = document.getElementById('meet-chat-input');
    if (!input || !input.value.trim()) return;

    const curUserName = (window.DokuHub && DokuHub.currentUser && DokuHub.currentUser.name)
      ? DokuHub.currentUser.name
      : 'User';
    const text = input.value.trim();
    input.value = '';

    const msgContainer = document.getElementById('meet-chat-messages');
    if (msgContainer) {
      const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      msgContainer.innerHTML += `
        <div style="background:#2563eb; padding:8px 12px; border-radius:10px 10px 2px 10px; align-self:flex-end; max-width:85%; box-shadow:0 2px 6px rgba(0,0,0,0.15);">
          <div style="font-size:10.5px; opacity:0.8; margin-bottom:2px; text-align:right;">${curUserName} • ${nowTime}</div>
          <div style="font-size:12.5px; color:#fff; word-break:break-word;">${text}</div>
        </div>
      `;
      msgContainer.scrollTop = msgContainer.scrollHeight;
    }

    this.messages.push({ from: curUserName, text, isSelf: true });
    await this.sendSignal('chat', text);
  },

  async sendSignal(type, data, toPeer = '') {
    if (!this.roomCode) return;
    try {
      const curUserName = (window.DokuHub && DokuHub.currentUser && DokuHub.currentUser.name)
        ? DokuHub.currentUser.name
        : 'User';
      await fetch('/api/meet/signal', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          room_code: this.roomCode,
          from_peer: this.peerId,
          to_peer: toPeer,
          type,
          data,
          user_name: curUserName
        })
      });
    } catch { }
  },

  async pollSignals() {
    if (!this.roomCode) return;
    try {
      const res = await fetch(`/api/meet/signals/${this.roomCode}?peer_id=${this.peerId}&since=${this.lastSignalId}`);
      if (res.ok) {
        const json = await res.json();

        // 1. Update active peers grid
        if (json.peers) {
          this.updateRemotePeersGrid(json.peers);
        }

        // 2. Process incoming signals (WebRTC handshake & non-looping chat)
        if (json.signals && Array.isArray(json.signals)) {
          for (const sig of json.signals) {
            // Track sequential signal id to avoid repeat fetching
            if (sig.id && sig.id > this.lastSignalId) {
              this.lastSignalId = sig.id;
            }

            // Deduplicate chat messages
            if (sig.type === 'chat') {
              if (this.renderedMessageIds.has(sig.id)) {
                continue;
              }
              this.renderedMessageIds.add(sig.id);

              const senderName = sig.user_name || sig.from_peer;
              const msgContainer = document.getElementById('meet-chat-messages');
              if (msgContainer) {
                const nowTime = new Date(sig.timestamp ? sig.timestamp * 1000 : Date.now()).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
                msgContainer.innerHTML += `
                  <div style="background:var(--sidebar-bg); border:1px solid var(--window-border); padding:8px 12px; border-radius:10px 10px 10px 2px; align-self:flex-start; max-width:85%; box-shadow:0 2px 6px rgba(0,0,0,0.08);">
                    <div style="font-size:10.5px; color:var(--accent-color); margin-bottom:2px; font-weight:600;">${senderName} • ${nowTime}</div>
                    <div style="font-size:12.5px; color:var(--text-main); word-break:break-word;">${sig.data}</div>
                  </div>
                `;
                msgContainer.scrollTop = msgContainer.scrollHeight;
              }
              this.messages.push({ from: senderName, text: sig.data, isSelf: false });
            } else {
              // WebRTC signaling: offer, answer, candidate, join, leave
              await this.handleIncomingSignal(sig);
            }
          }
        }
      }
    } catch { }
  },

  copyRoomLink() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const link = `${window.location.origin}/?meet=${this.roomCode}`;
    navigator.clipboard.writeText(link);
    if (window.DokuIsland) {
      DokuIsland.success(
        isId ? `Tautan rapat disalin: ${this.roomCode}` : `Meeting link copied: ${this.roomCode}`,
        'BijiMeet'
      );
    }
  },

  cleanupMedia() {
    if (this.vadTimer) {
      cancelAnimationFrame(this.vadTimer);
      this.vadTimer = null;
    }
    if (this.audioCtx && this.audioCtx.state !== 'closed') {
      try { this.audioCtx.close(); } catch { }
      this.audioCtx = null;
    }
    if (this.localStream) {
      this.localStream.getTracks().forEach(t => t.stop());
      this.localStream = null;
    }
    if (this.screenStream) {
      this.screenStream.getTracks().forEach(t => t.stop());
      this.screenStream = null;
    }

    // Close all WebRTC PeerConnections
    Object.values(this.peerConnections).forEach(pc => {
      try { pc.close(); } catch { }
    });
    this.peerConnections = {};
    this.remoteStreams = {};
    this.remoteAnalysers = {};

    // Remove any audio sink elements
    document.querySelectorAll('audio[id^="audio-sink-"]').forEach(el => el.remove());
  },

  leaveMeeting() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }

    // Send leave signal to server
    this.sendSignal('leave', 'leave_room');

    this.cleanupMedia();

    if (window.DokuIsland) {
      DokuIsland.info(isId ? 'Anda meninggalkan rapat' : 'You left the meeting', 'BijiMeet');
    }

    this.renderLobby();
  }
};
