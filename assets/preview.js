(function () {
  'use strict';

  var COPIED_MS = 1500;

  function fallbackCopy(text) {
    var area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.top = '0';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    var ok = false;
    try {
      ok = document.execCommand('copy');
    } catch (e) {
      ok = false;
    }
    document.body.removeChild(area);
    return ok ? Promise.resolve() : Promise.reject(new Error('copy failed'));
  }

  function writeClipboard(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).catch(function () {
        return fallbackCopy(text);
      });
    }
    return fallbackCopy(text);
  }

  function markCopied(button) {
    button.setAttribute('data-copied', 'true');
    window.setTimeout(function () {
      button.removeAttribute('data-copied');
    }, COPIED_MS);
  }

  function bindCopy(button, getText) {
    button.addEventListener('click', function () {
      writeClipboard(getText()).then(function () {
        markCopied(button);
      }, function () {});
    });
  }

  function setupCopyButtons() {
    document.querySelectorAll('.copy-button[data-copy]').forEach(function (button) {
      bindCopy(button, function () {
        return button.getAttribute('data-copy') || '';
      });
    });
  }

  // コードブロックの帯に出す言語名。生のHTMLは html として示す。
  function codeLanguage(block) {
    if (block.classList.contains('raw-html')) {
      return 'html';
    }
    var code = block.querySelector('code');
    var match = code && /(?:^|\s)language-(\S+)/.exec(code.className);
    return match ? match[1] : '';
  }

  function setupCodeCopy() {
    document.querySelectorAll('pre.code-block').forEach(function (block) {
      var language = codeLanguage(block);
      if (language) {
        block.setAttribute('data-lang', language);
      }
      var button = document.createElement('button');
      button.type = 'button';
      button.className = 'copy-button code-copy';
      button.setAttribute('aria-label', 'コードをコピー');
      bindCopy(button, function () {
        var code = block.querySelector('code');
        return (code || block).textContent || '';
      });
      block.appendChild(button);
    });
  }

  function cssValue(name) {
    return window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  // base テーマのときに、図の色をページの色へ合わせる。
  function mermaidThemeVariables() {
    var bg = cssValue('--mb-color-bg');
    var surface = cssValue('--mb-color-surface');
    var text = cssValue('--mb-color-text');
    var muted = cssValue('--mb-color-muted');
    var border = cssValue('--mb-color-border');
    var accent = cssValue('--mb-color-accent');
    var accentSoft = cssValue('--mb-color-accent-soft');
    return {
      fontFamily: cssValue('--mb-font-body'),
      background: bg,
      primaryColor: accentSoft,
      primaryTextColor: text,
      primaryBorderColor: accent,
      secondaryColor: surface,
      secondaryTextColor: text,
      secondaryBorderColor: border,
      tertiaryColor: bg,
      tertiaryTextColor: text,
      tertiaryBorderColor: border,
      lineColor: muted,
      textColor: text,
      edgeLabelBackground: bg,
      clusterBkg: surface,
      clusterBorder: border,
      noteBkgColor: surface,
      noteTextColor: text,
      noteBorderColor: border
    };
  }

  function setupMermaid() {
    var blocks = document.querySelectorAll('pre.mermaid');
    if (!window.mermaid || blocks.length === 0) {
      return;
    }
    try {
      var theme = document.body.dataset.mermaidTheme;
      var options = {
        startOnLoad: false,
        securityLevel: 'strict',
        theme: theme,
        fontFamily: cssValue('--mb-font-body'),
        // 日本語のラベルが数文字で折り返されないよう、箱の中の幅を広げる。
        flowchart: { wrappingWidth: 240 }
      };
      if (theme === 'base') {
        options.themeVariables = mermaidThemeVariables();
      }
      window.mermaid.initialize(options);
      var result = window.mermaid.run({ querySelector: 'pre.mermaid' });
      if (result && typeof result.catch === 'function') {
        result.catch(function () {});
      }
    } catch (e) {
      // 図の描画に失敗しても、ほかの部分は動かす。
    }
  }

  // 留めた見出しの上の余白（px）。
  var PIN_PAD = 8;

  // 留める行に入れる、見出しの中身の写し。リンクとボタンは働かせない。
  function pinnedLine(heading) {
    var line = document.createElement('div');
    line.className = 'pinned-line';
    heading.childNodes.forEach(function (node) {
      line.appendChild(node.cloneNode(true));
    });
    line.querySelectorAll('[id]').forEach(function (el) {
      el.removeAttribute('id');
    });
    line.querySelectorAll('button').forEach(function (el) {
      el.remove();
    });
    line.querySelectorAll('a').forEach(function (el) {
      var span = document.createElement('span');
      while (el.firstChild) {
        span.appendChild(el.firstChild);
      }
      el.replaceWith(span);
    });
    return line;
  }

  // 読んでいる位置の見出しと、その親の見出しを、画面の上端に重ねて留める。
  // 次の見出しが届いたら、入れ替わる行を上へ押し出す。
  function setupPinnedHeadings() {
    var doc = document.querySelector('.document');
    if (!doc) {
      return;
    }
    var heads = [];
    Array.prototype.forEach.call(doc.children, function (el) {
      var match = /^H([1-6])$/.exec(el.tagName);
      if (!match) {
        return;
      }
      var head = { el: el, level: Number(match[1]), path: null, line: null, top: 0 };
      var parent = null;
      for (var i = heads.length - 1; i >= 0; i--) {
        if (heads[i].level < head.level) {
          parent = heads[i];
          break;
        }
      }
      head.path = (parent ? parent.path : []).concat([head]);
      heads.push(head);
    });
    if (heads.length === 0) {
      return;
    }

    var bar = document.createElement('div');
    bar.className = 'pinned';
    bar.hidden = true;
    bar.setAttribute('aria-hidden', 'true');
    var inner = document.createElement('div');
    inner.className = 'pinned-inner';
    bar.appendChild(inner);
    document.body.appendChild(bar);

    // 1 行の高さは CSS が決める。見出しの文字が、留める前と同じ位置に来るように、行の中での文字の下がり幅を求める。
    var probe = document.createElement('div');
    probe.className = 'pinned-line';
    inner.appendChild(probe);
    bar.hidden = false;
    var lineHeight = probe.getBoundingClientRect().height || 24;
    bar.hidden = true;
    inner.removeChild(probe);
    var headingStyle = window.getComputedStyle(heads[0].el);
    var headingLine = parseFloat(headingStyle.lineHeight) || parseFloat(headingStyle.fontSize) * 1.3;
    var textOffset = Math.max(0, (lineHeight - headingLine) / 2);

    // 見出しが留まる位置。親の行の数だけ下がる。
    function slotTop(head) {
      return PIN_PAD + (head.path.length - 1) * lineHeight;
    }

    function buildLine(head) {
      var line = pinnedLine(head.el);
      line.addEventListener('click', function () {
        var target = slotTop(head) + textOffset;
        window.scrollBy(0, head.el.getBoundingClientRect().top - target + 1);
      });
      return line;
    }

    var shown = null;
    var waiting = false;

    function update() {
      waiting = false;
      var current = null;
      for (var i = 0; i < heads.length; i++) {
        var head = heads[i];
        head.top = head.el.getBoundingClientRect().top - textOffset;
        if (head.top <= slotTop(head)) {
          current = i;
        }
      }
      if (current === null) {
        bar.hidden = true;
        shown = null;
        return;
      }

      var stack = heads[current].path;
      var count = stack.length;
      if (shown !== heads[current]) {
        shown = heads[current];
        inner.textContent = '';
        stack.forEach(function (head, index) {
          if (!head.line) {
            head.line = buildLine(head);
          }
          head.line.style.top = (PIN_PAD + index * lineHeight) + 'px';
          head.line.style.zIndex = String(count - index);
          inner.appendChild(head.line);
        });
      }

      // 次の見出しの親にあたる行は残し、それより深い行を、次の見出しに押される分だけ上げる。
      var keep = count;
      var shift = 0;
      var next = heads[current + 1];
      if (next) {
        keep = Math.min(count, next.path.length - 1);
        shift = PIN_PAD + count * lineHeight - next.top;
        shift = Math.max(0, Math.min(shift, (count - keep) * lineHeight));
      }
      stack.forEach(function (head, index) {
        head.line.style.transform = index >= keep && shift > 0 ? 'translateY(' + (-shift) + 'px)' : '';
      });
      bar.style.height = Math.round(PIN_PAD + count * lineHeight - shift + textOffset) + 'px';
      bar.hidden = false;
    }

    function requestUpdate() {
      if (!waiting) {
        waiting = true;
        window.requestAnimationFrame(update);
      }
    }

    window.addEventListener('scroll', requestUpdate, { passive: true });
    window.addEventListener('resize', requestUpdate);
    window.addEventListener('load', requestUpdate);
    update();
  }

  setupCopyButtons();
  setupCodeCopy();
  setupMermaid();
  setupPinnedHeadings();
})();
