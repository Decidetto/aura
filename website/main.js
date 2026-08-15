// Interactive behaviors for Aura Website

document.addEventListener('DOMContentLoaded', () => {
  setupSpotlightGlow();
  setupSmoothScrolling();
  setupIntersectionObserver();
  setupMockSettings();
  startTypingSimulator();
  setupRevealOnScroll();
  setupScrollListeners();
  setupFAQAccordions();
  setupPlatformSpecificDownloads();
  setupConfirmDownloadModal();
});

/**
 * Creates a dynamic spotlight glow effect inside bento cards following the cursor
 */
function setupSpotlightGlow() {
  const cards = document.querySelectorAll('.bento-card');
  const bgContainer = document.querySelector('.aura-bg-container');
  
  // Local card spotlights
  cards.forEach(card => {
    card.addEventListener('mousemove', (e) => {
      const rect = card.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      
      card.style.setProperty('--mouse-x', `${x}px`);
      card.style.setProperty('--mouse-y', `${y}px`);
    });
  });

  // Global background spotlight
  if (bgContainer) {
    window.addEventListener('mousemove', (e) => {
      bgContainer.style.setProperty('--mouse-x', `${e.clientX}px`);
      bgContainer.style.setProperty('--mouse-y', `${e.clientY}px`);
    });
  }
}

/**
 * Handles smooth scrolling and updates the active nav link on click
 */
function setupSmoothScrolling() {
  const navLinks = document.querySelectorAll('.nav-link');
  
  navLinks.forEach(link => {
    link.addEventListener('click', (e) => {
      const targetId = link.getAttribute('href');
      
      // Let standard navigation happen for page redirects (like lang switchers)
      if (!targetId || !targetId.startsWith('#')) {
        return;
      }
      
      e.preventDefault();
      
      if (targetId === '#') {
        window.scrollTo({ top: 0, behavior: 'smooth' });
        return;
      }
      
      const targetSection = document.querySelector(targetId);
      if (targetSection) {
        const offset = 72; // height of fixed header
        const targetPosition = targetSection.getBoundingClientRect().top + window.scrollY - offset;
        
        window.scrollTo({
          top: targetPosition,
          behavior: 'smooth'
        });
      }
    });
  });
}

/**
 * Highlights active navigation link based on scroll position
 */
function setupIntersectionObserver() {
  const sections = document.querySelectorAll('section[id]');
  const navLinks = document.querySelectorAll('.nav-link');
  
  const options = {
    root: null,
    rootMargin: '-80px 0px -60% 0px', // check when section covers top portion of viewport
    threshold: 0
  };
  
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const id = entry.target.getAttribute('id');
        
        navLinks.forEach(link => {
          if (link.getAttribute('href') === `#${id}`) {
            link.classList.add('active');
          } else {
            link.classList.remove('active');
          }
        });
      }
    });
  }, options);
  
  sections.forEach(section => observer.observe(section));
}

/**
 * Controls the live interactive Mock Settings panel
 */
function setupMockSettings() {
  const tabs = document.querySelectorAll('[data-mock-tab]');
  const panels = document.querySelectorAll('.mock-panel');
  const slider = document.getElementById('mock-sound-vol');
  const sliderVal = document.getElementById('mock-vol-val');
  
  const radioCloud = document.getElementById('mock-radio-cloud');
  const radioLocal = document.getElementById('mock-radio-local');
  const localModelCard = document.getElementById('mock-card-local-model');
  const cloudApiCard = document.getElementById('mock-card-api-keys');
  const modelCards = document.querySelectorAll('.mock-model-card');
  
  const apiProviderSelect = document.getElementById('mock-api-provider');
  const apiKeyInput = document.getElementById('mock-api-key');
  
  const clearHistoryBtn = document.getElementById('mock-btn-clear-history');
  const historyList = document.getElementById('mock-history-list');
  const historyEmpty = document.getElementById('mock-history-empty');
  const historySearch = document.getElementById('mock-history-search');
  const filterBtns = document.querySelectorAll('.mock-filter-btn');
  
  // Tab switching for all 5 tabs
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const targetPanelId = `mock-panel-${tab.getAttribute('data-mock-tab')}`;
      
      tabs.forEach(t => {
        t.classList.remove('mock-tab-active');
        t.setAttribute('aria-selected', 'false');
      });
      tab.classList.add('mock-tab-active');
      tab.setAttribute('aria-selected', 'true');
      
      panels.forEach(panel => {
        if (panel.getAttribute('id') === targetPanelId) {
          panel.style.display = 'flex';
        } else {
          panel.style.display = 'none';
        }
      });
    });
  });

  // Slider value updates
  if (slider && sliderVal) {
    slider.addEventListener('input', (e) => {
      sliderVal.textContent = `${e.target.value}%`;
    });
  }

  // Toggling Local vs Cloud settings cards inside Speech tab
  if (radioCloud && radioLocal) {
    const handleEngineChange = () => {
      if (radioLocal.checked) {
        if (localModelCard) localModelCard.style.display = 'block';
        if (cloudApiCard) cloudApiCard.style.display = 'none';
      } else {
        if (localModelCard) localModelCard.style.display = 'none';
        if (cloudApiCard) cloudApiCard.style.display = 'block';
      }
    };
    
    radioCloud.addEventListener('change', handleEngineChange);
    radioLocal.addEventListener('change', handleEngineChange);
  }

  // Local model cards selection toggler
  modelCards.forEach(card => {
    card.addEventListener('click', () => {
      modelCards.forEach(c => c.classList.remove('mock-model-active'));
      card.classList.add('mock-model-active');
    });
  });

  // Dynamic API Key input placeholder based on provider selection
  if (apiProviderSelect && apiKeyInput) {
    const isRu = document.documentElement.lang === 'ru';
    apiProviderSelect.addEventListener('change', (e) => {
      const provider = e.target.value;
      if (provider === 'gemini') {
        apiKeyInput.placeholder = isRu ? 'Введите ваш API-ключ Gemini...' : 'Enter your Gemini API key...';
      } else if (provider === 'openai') {
        apiKeyInput.placeholder = isRu ? 'Введите ваш API-ключ OpenAI...' : 'Enter your OpenAI API key...';
      } else if (provider === 'groq') {
        apiKeyInput.placeholder = isRu ? 'Введите ваш API-ключ Groq...' : 'Enter your Groq API key...';
      } else if (provider === 'huggingface') {
        apiKeyInput.placeholder = isRu ? 'Введите ваш токен Hugging Face...' : 'Enter your Hugging Face token...';
      } else if (provider === 'custom') {
        apiKeyInput.placeholder = isRu ? 'https://your-server.com/v1' : 'https://your-server.com/v1';
      }
    });
  }

  // History filtering by type (All, Local, Cloud)
  let activeFilter = 'all';
  let searchQuery = '';

  function updateHistoryView() {
    if (!historyList) return;
    const items = historyList.querySelectorAll('.mock-history-item');
    let visibleCount = 0;

    items.forEach(item => {
      const source = item.getAttribute('data-source');
      const text = (item.querySelector('.mock-history-text')?.textContent || '').toLowerCase();
      
      const matchesFilter = (activeFilter === 'all' || source === activeFilter);
      const matchesSearch = (!searchQuery || text.includes(searchQuery));

      if (matchesFilter && matchesSearch) {
        item.style.display = 'block';
        visibleCount++;
      } else {
        item.style.display = 'none';
      }
    });

    if (historyEmpty) {
      historyEmpty.style.display = visibleCount === 0 ? 'block' : 'none';
    }
  }

  filterBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      filterBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      activeFilter = btn.getAttribute('data-filter') || 'all';
      updateHistoryView();
    });
  });

  if (historySearch) {
    historySearch.addEventListener('input', (e) => {
      searchQuery = e.target.value.trim().toLowerCase();
      updateHistoryView();
    });
  }

  // Mock clearing history
  if (clearHistoryBtn && historyList && historyEmpty) {
    clearHistoryBtn.addEventListener('click', () => {
      historyList.style.display = 'none';
      historyEmpty.style.display = 'block';
      clearHistoryBtn.style.display = 'none';
    });
  }
}

/**
 * Runs the automatic typing simulator demonstrating Aura's core features
 */
function startTypingSimulator() {
  const editorText = document.getElementById('demo-editor-text');
  const overlayPill = document.getElementById('mock-overlay-pill');
  const overlayStatus = document.getElementById('mock-overlay-status');
  const soundBars = document.querySelectorAll('#mock-overlay-pill .sound-bar');
  
  if (!editorText || !overlayPill || !overlayStatus) return;

  const isRu = document.documentElement.lang === 'ru';
  const rawPhrase = isRu 
    ? "Напиши функцию для быстрой сортировки логов на Rust" 
    : "Write a function for fast log sorting in Rust";
  const cleanPhrase = isRu 
    ? "Напиши функцию для быстрой сортировки логов на Rust." 
    : "Write a function for fast log sorting in Rust.";
  
  let breathingInterval = null;
  let timerInterval = null;

  function stopAllAnimations() {
    if (breathingInterval) clearInterval(breathingInterval);
    if (timerInterval) clearInterval(timerInterval);
    
    soundBars.forEach(bar => {
      bar.removeAttribute("style");
      bar.setAttribute("height", "6");
      bar.setAttribute("y", "11.6001");
    });
  }

  async function runSimulationLoop() {
    stopAllAnimations();

    // Reset editor state
    editorText.textContent = "";
    editorText.style.color = 'var(--text-primary)';
    overlayPill.classList.remove('active', 'processing');
    overlayStatus.textContent = "0:00";
    
    await sleep(1500);
    
    // Step 1: Open Dictation Pill Overlay (Start recording state)
    overlayPill.classList.add('active');
    
    let seconds = 0;
    timerInterval = setInterval(() => {
      seconds++;
      overlayStatus.textContent = `0:0${seconds}`;
    }, 1000);
    
    await sleep(800);
    
    // Step 2: Speak / dictate phrase
    for (let i = 0; i < rawPhrase.length; i++) {
      editorText.textContent += rawPhrase[i];
      await sleep(35 + Math.random() * 50);
    }
    
    clearInterval(timerInterval);
    await sleep(350);
    
    // Step 3: Transcription processing state
    overlayPill.classList.add('processing');
    overlayStatus.textContent = isRu ? "Вставка…" : "Transcribing...";
    
    let angle = 0;
    breathingInterval = setInterval(() => {
      soundBars.forEach((bar, index) => {
        const distFromCenter = Math.abs(index - 4);
        const h = 5 + Math.sin(angle - distFromCenter * 0.45) * 5.5;
        const y = 14.6 - (h / 2);
        bar.setAttribute("height", h.toString());
        bar.setAttribute("y", y.toString());
      });
      angle += 0.05;
    }, 16);
    
    await sleep(1000);
    
    // Step 4: Punctuation and insertion completed
    editorText.textContent = cleanPhrase;
    editorText.style.color = 'var(--text-primary)';
    
    stopAllAnimations();
    overlayPill.classList.remove('active', 'processing');
    
    // Pause before repeating
    await sleep(4000);
    runSimulationLoop();
  }

  function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  runSimulationLoop();
}



/**
 * Triggers smooth entry transitions for key components as they enter the viewport
 */
function setupRevealOnScroll() {
  const reveals = document.querySelectorAll('.reveal-on-scroll');
  
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('revealed');
        observer.unobserve(entry.target); // Trigger once
      }
    });
  }, {
    root: null,
    threshold: 0.05,
    rootMargin: '0px 0px -50px 0px'
  });
  
  reveals.forEach(el => observer.observe(el));
}

/**
 * Handles scroll-dependent visibility for header buttons and the scroll-to-top button
 */
function setupScrollListeners() {
  const headerActions = document.querySelector('.site-header .header-actions');
  const scrollTopBtn = document.getElementById('scroll-top-btn');
  
  const handleScroll = () => {
    const scrollPos = window.scrollY;
    
    if (scrollPos > 400) {
      if (headerActions) headerActions.classList.add('visible');
      if (scrollTopBtn) scrollTopBtn.classList.add('visible');
    } else {
      if (headerActions) headerActions.classList.remove('visible');
      if (scrollTopBtn) scrollTopBtn.classList.remove('visible');
    }
  };
  
  window.addEventListener('scroll', handleScroll);
  
  // Smoothly scroll back to top on click
  if (scrollTopBtn) {
    scrollTopBtn.addEventListener('click', () => {
      window.scrollTo({
        top: 0,
        behavior: 'smooth'
      });
    });
  }
}

/**
 * Handles independent folding accordion drawers for the FAQ block
 */
function setupFAQAccordions() {
  const faqItems = document.querySelectorAll('.faq-item');
  
  faqItems.forEach(item => {
    const btn = item.querySelector('.faq-question-btn');
    const pane = item.querySelector('.faq-answer-pane');
    
    if (!btn || !pane) return;
    
    btn.addEventListener('click', () => {
      const isActive = item.classList.contains('faq-active');
      
      // Close other items (classic accordion logic)
      faqItems.forEach(otherItem => {
        if (otherItem !== item) {
          otherItem.classList.remove('faq-active');
          const otherBtn = otherItem.querySelector('.faq-question-btn');
          const otherPane = otherItem.querySelector('.faq-answer-pane');
          if (otherBtn) otherBtn.setAttribute('aria-expanded', 'false');
          if (otherPane) otherPane.style.maxHeight = '0';
        }
      });
      
      // Toggle current item
      if (isActive) {
        item.classList.remove('faq-active');
        btn.setAttribute('aria-expanded', 'false');
        pane.style.maxHeight = '0';
      } else {
        item.classList.add('faq-active');
        btn.setAttribute('aria-expanded', 'true');
        pane.style.maxHeight = `${pane.scrollHeight}px`;
      }
    });
  });
}

function setupPlatformSpecificDownloads() {
  const isRu = document.documentElement.lang === 'ru';
  const isWindows = /Windows|Win32|Win64|wintarget/i.test(navigator.userAgent || navigator.platform);
  
  const heroSubtext = document.getElementById('hero-download-subtext');
  
  if (heroSubtext) {
    if (isWindows) {
      heroSubtext.textContent = isRu 
        ? "(Windows 10 и 11)" 
        : "(Windows 10 & 11)";
      heroSubtext.style.color = ""; // reset to default CSS opacity
    } else {
      heroSubtext.textContent = isRu 
        ? "(требуется Windows 10 или 11)" 
        : "(requires Windows 10 or 11)";
      heroSubtext.style.color = "rgba(255, 66, 0, 0.55)"; // highlight warning slightly
    }
  }
}

/**
 * Sets up the Confirm Download Modal dialog and binds it to download triggers.
 */
function setupConfirmDownloadModal() {
  const modal = document.getElementById('download-modal');
  if (!modal) return;

  const cancelBtn = document.getElementById('modal-cancel-btn');
  const confirmBtn = document.getElementById('modal-confirm-btn');
  const downloadLinks = document.querySelectorAll('#hero-download-btn, #header-download-btn, #footer-download-link');

  let activeUrl = '';

  // Open modal handler
  function openModal(e, url) {
    e.preventDefault();
    activeUrl = url;
    if (confirmBtn) {
      confirmBtn.setAttribute('href', activeUrl);
    }
    document.body.classList.add('modal-active');
    modal.classList.remove('exiting');
    modal.classList.add('active');
    
    // Focus Cancel button for accessibility
    if (cancelBtn) {
      cancelBtn.focus();
    }
  }

  // Close modal handler with asymmetric exit timing (snap down animation)
  function closeModal() {
    modal.classList.add('exiting');
    
    // Wait for the exit animation (120ms) before hiding overlay
    setTimeout(() => {
      modal.classList.remove('active');
      modal.classList.remove('exiting');
      document.body.classList.remove('modal-active');
      activeUrl = '';
    }, 120);
  }

  // Bind download buttons
  downloadLinks.forEach(link => {
    link.addEventListener('click', (e) => {
      const url = link.getAttribute('href');
      if (url && url !== '#') {
        openModal(e, url);
      }
    });
  });

  // Bind cancel and confirm actions
  if (cancelBtn) {
    cancelBtn.addEventListener('click', closeModal);
  }

  if (confirmBtn) {
    confirmBtn.addEventListener('click', () => {
      // Small timeout to allow active transition feel before redirect
      setTimeout(closeModal, 100);
    });
  }

  // Close on overlay click
  modal.addEventListener('click', (e) => {
    if (e.target === modal) {
      closeModal();
    }
  });

  // Close on ESC key press
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal.classList.contains('active')) {
      closeModal();
    }
  });

  // Trap focus inside modal for keyboard accessibility
  modal.addEventListener('keydown', (e) => {
    if (e.key === 'Tab' && modal.classList.contains('active')) {
      const focusables = modal.querySelectorAll('button, a[href]');
      if (focusables.length === 0) return;
      const firstFocusable = focusables[0];
      const lastFocusable = focusables[focusables.length - 1];

      if (e.shiftKey) { // Shift + Tab
        if (document.activeElement === firstFocusable) {
          e.preventDefault();
          lastFocusable.focus();
        }
      } else { // Tab
        if (document.activeElement === lastFocusable) {
          e.preventDefault();
          firstFocusable.focus();
        }
      }
    }
  });
}


