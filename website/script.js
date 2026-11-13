"use strict";

(() => {
  const repository = "https://github.com/linventif/open-process";
  const releasesUrl = `${repository}/releases`;
  const strings = {
    en: {
      title: "Open Process — Model. Understand. Explore.",
      description: "A free, open-source workspace for BPMN, Petri nets and process discovery. Built in Rust for Windows, macOS, Linux and the web.",
      skip: "Skip to content", navLabel: "Main navigation", navExplore: "Explore", navLearn: "Learn by doing", getApp: "Get the app",
      heroEyebrow: "Free software. Clearer processes.", heroLine1: "Every process.", heroLine2: "One workspace",
      heroDescription: "From the first BPMN diagram to the last matrix. Model, simulate and understand your processes in one open-source tool.",
      downloadFree: "Download for free", seeInside: "Take a look inside", platformNote: "Windows, macOS, Linux & the web. No account needed.",
      noteLine1: "Draw the flow.", noteLine2: "Understand the system.", builtFor: "Built for curious minds & complex workflows.",
      exploreLabel: "Explore the application", viewsLabel: "Application views", petriTab: "Petri nets", miningTab: "Process mining", splitTab: "Split views", realApp: "The actual app", expandLabel: "Enlarge application screenshot", enlarge: "Enlarge", closeLabel: "Close screenshot",
      featuresEyebrow: "THE WHOLE PICTURE", featuresLine1: "Less tool switching.", featuresLine2: "More understanding.", featuresIntro: "The diagram, the maths and the discovery belong together. So we put them together.",
      modelEyebrow: "01 — MODEL", analyseEyebrow: "02 — ANALYSE", discoverEyebrow: "03 — DISCOVER",
      modelTitle: "Make the flow visible.", modelBody: "Build BPMN 2.0 diagrams with pools, lanes, events and gateways. Drag elements into place and catch modelling issues as you go.", modelPoint1: "BPMN 2.0 XML import & export", modelPoint2: "BPMN ↔ Petri net conversion", modelPoint3: "Light, dark & hand-drawn styles",
      analyseTitle: "Go beyond the diagram.", analyseBody: "Play the tokens, inspect the matrices and explore reachable states. Find out how your Petri net actually behaves.", analysePoint1: "Pre, Post & incidence matrices", analysePoint2: "Invariants, deadlocks & soundness", analysePoint3: "PNML, LaTeX & CSV exports",
      discoverTitle: "Let the traces speak.", discoverBody: "Start with an event log. Explore the relations, follow the Alpha algorithm and turn observed behaviour into a Petri net.", discoverPoint1: "Text, CSV & XES event logs", discoverPoint2: "Footprint matrix & Alpha steps", discoverPoint3: "Replay traces on the discovered net",
      learnEyebrow: "MADE FOR THE AHA MOMENT", learnLine1: "Your turn to", learnLine2: "connect the dots", learnIntro: "For the lecture, the exercise and the moment it finally clicks. Choose how much help you want.", learnLink: "Explore the learning tools",
      step1Title: "Work it out yourself.", step1Body: "Fill in Pre, Post and incidence matrices, or build your footprint matrix and Alpha sets by hand.",
      step2Title: "Check your reasoning.", step2Body: "Compare your answers, spot missing or extra items, and use hints to move forward.",
      step3Title: "See the result come alive.", step3Body: "Generate a net from your answers. Replay the traces, fire transitions and explore the analysis alongside your model.", learnFooter: "From a formula on the board to a model you can explore.",
      downloadEyebrow: "YOUR WORKSPACE, YOUR CHOICE", downloadTitle: "Make yourself at home.", downloadIntro: "A native app for your desktop. A WebAssembly build for your browser. The same tools, wherever you work.",
      allReleases: "All releases", viewReleases: "View releases", download: "Download", macArchitecture: "Mac processor", macArm: "Apple Silicon · M series",
      windowsNote: "A standalone executable. Download, open, model.", macNote: "Native builds for Apple Silicon and Intel Macs.", linuxNote: "A native binary for your favourite distribution.",
      browserTitle: "More of a browser person?", browserBody: "Open the cloud version at open-process.linv.dev. Your session stays in your browser.", browserLink: "Open the cloud app",
      sourceEyebrow: "OPEN IN EVERY SENSE", sourceTitle: "Yours to use. Ours to build.", sourceBody: "Written in Rust with egui. Free under MIT or Apache 2.0. Read the code, report a bug or help shape what comes next.", sourceLink: "Explore the source", issueLink: "Share an idea", terminalTitle: "Start from source", terminalComment: "Start with Rust and the system dependencies below.", buildInstructions: "Build instructions & system dependencies ↗", footerText: "A little more clarity. A lot more possibility.",
      releaseInitial: "Desktop binaries are published on GitHub Releases. Build from source if no release is available yet.",
      releaseNone: "No public release is available yet. You can already build the app from source below.",
      releaseError: "Release information is unavailable. Check GitHub for downloads, or build from source below.",
      releaseAvailable: "Latest release: {version}. Available binaries are linked below.",
      bpmnCaption: "An idea becomes a process. Pools, lanes, gateways — all in their place.", bpmnAlt: "Open Process BPMN editor showing an order handling process with pools, lanes and gateways",
      petriCaption: "From tokens to equations. Your Petri net and its matrices, side by side.", petriAlt: "Petri net for mutual exclusion with tokens, rectangular transitions and Pre, Post and incidence matrices",
      miningCaption: "From event logs to relations. Follow the Alpha algorithm, one step at a time.", miningAlt: "Process discovery workspace with an event log, footprint matrix and Alpha algorithm",
      splitCaption: "Make room for your reasoning. A BPMN diagram and Petri matrices in split panes.", splitAlt: "Split workspace showing a BPMN model on the left and Petri net matrices on the right"
    },
    fr: {
      title: "Open Process — Modéliser. Comprendre. Explorer.",
      description: "Un espace libre et gratuit pour les BPMN, réseaux de Petri et la découverte de processus. En Rust, sur Windows, macOS, Linux et navigateur.",
      skip: "Aller au contenu", navLabel: "Navigation principale", navExplore: "Découvrir", navLearn: "Apprendre", getApp: "Télécharger",
      heroEyebrow: "Logiciel libre. Processus plus clairs.", heroLine1: "Vos processus.", heroLine2: "Un seul espace",
      heroDescription: "Du premier diagramme BPMN à la dernière matrice. Modélisez, simulez et comprenez vos processus dans un seul outil open source.",
      downloadFree: "Télécharger gratuitement", seeInside: "Voir l’application", platformNote: "Windows, macOS, Linux et navigateur. Sans compte.",
      noteLine1: "Dessiner le flux.", noteLine2: "Comprendre le système.", builtFor: "Pour les esprits curieux et les processus complexes.",
      exploreLabel: "Découvrir l’application", viewsLabel: "Vues de l’application", petriTab: "Réseaux de Petri", miningTab: "Process mining", splitTab: "Vues partagées", realApp: "La vraie application", expandLabel: "Agrandir la capture de l’application", enlarge: "Agrandir", closeLabel: "Fermer la capture",
      featuresEyebrow: "UNE VUE D’ENSEMBLE", featuresLine1: "Moins de changements d’outil.", featuresLine2: "Plus de compréhension.", featuresIntro: "Le diagramme, les maths et la découverte vont ensemble. Alors on les a réunis.",
      modelEyebrow: "01 — MODÉLISER", analyseEyebrow: "02 — ANALYSER", discoverEyebrow: "03 — DÉCOUVRIR",
      modelTitle: "Donnez forme aux flux.", modelBody: "Construisez vos BPMN 2.0 avec pools, couloirs, événements et passerelles. Glissez les éléments en place et repérez les erreurs de modélisation.", modelPoint1: "Import et export XML BPMN 2.0", modelPoint2: "Conversion BPMN ↔ réseau de Petri", modelPoint3: "Styles clair, sombre et croquis",
      analyseTitle: "Allez au-delà du dessin.", analyseBody: "Jouez les jetons, examinez les matrices et explorez les états accessibles. Comprenez le comportement de votre réseau de Petri.", analysePoint1: "Matrices Pre, Post et d’incidence", analysePoint2: "Invariants, blocages et soundness", analysePoint3: "Exports PNML, LaTeX et CSV",
      discoverTitle: "Faites parler les traces.", discoverBody: "Partez d’un journal d’événements. Explorez les relations, suivez l’algorithme Alpha et transformez les comportements observés en réseau de Petri.", discoverPoint1: "Journaux texte, CSV et XES", discoverPoint2: "Matrice des relations et étapes Alpha", discoverPoint3: "Rejeu des traces sur le réseau découvert",
      learnEyebrow: "POUR LE DÉCLIC", learnLine1: "À vous de", learnLine2: "relier les points", learnIntro: "Pour le cours, l’exercice et le moment où tout s’éclaire. Choisissez le niveau d’aide qui vous convient.", learnLink: "Découvrir les outils pédagogiques",
      step1Title: "Cherchez par vous-même.", step1Body: "Remplissez les matrices Pre, Post et d’incidence, ou construisez votre matrice des relations et vos ensembles Alpha à la main.",
      step2Title: "Vérifiez votre raisonnement.", step2Body: "Comparez vos réponses, repérez les éléments manquants ou en trop et avancez grâce aux indices.",
      step3Title: "Voyez le résultat prendre vie.", step3Body: "Générez un réseau à partir de vos réponses. Rejouez les traces, franchissez les transitions et affichez l’analyse à côté du modèle.", learnFooter: "De la formule au tableau au modèle que l’on explore.",
      downloadEyebrow: "VOTRE ESPACE, VOTRE CHOIX", downloadTitle: "Installez-vous.", downloadIntro: "Une application native sur votre ordinateur. Une version WebAssembly dans votre navigateur. Les mêmes outils, où que vous travailliez.",
      allReleases: "Toutes les versions", viewReleases: "Voir les versions", download: "Télécharger", macArchitecture: "Processeur du Mac", macArm: "Apple Silicon · série M",
      windowsNote: "Un exécutable autonome. Téléchargez, ouvrez, modélisez.", macNote: "Des versions natives pour les Mac Apple Silicon et Intel.", linuxNote: "Un binaire natif pour votre distribution préférée.",
      browserTitle: "Plutôt navigateur ?", browserBody: "Ouvrez la version cloud sur open-process.linv.dev. Votre session reste dans votre navigateur.", browserLink: "Ouvrir la version cloud",
      sourceEyebrow: "OUVERT, DANS TOUS LES SENS", sourceTitle: "Libre à vous. À construire ensemble.", sourceBody: "Écrit en Rust avec egui. Libre sous licence MIT ou Apache 2.0. Explorez le code, signalez un bug ou participez à la suite.", sourceLink: "Explorer le code", issueLink: "Proposer une idée", terminalTitle: "Partir du code source", terminalComment: "Installez Rust et les dépendances système ci-dessous.", buildInstructions: "Compilation et dépendances système ↗", footerText: "Un peu plus de clarté. Beaucoup plus de possibilités.",
      releaseInitial: "Les binaires sont publiés sur GitHub Releases. Compilez le code si aucune version n’est encore disponible.",
      releaseNone: "Aucune version publique disponible pour le moment. Vous pouvez déjà compiler l’application ci-dessous.",
      releaseError: "Les informations de version sont indisponibles. Consultez GitHub ou compilez le code ci-dessous.",
      releaseAvailable: "Dernière version : {version}. Les binaires disponibles sont accessibles ci-dessous.",
      bpmnCaption: "Une idée devient un processus. Pools, couloirs, passerelles : tout trouve sa place.", bpmnAlt: "Éditeur BPMN Open Process présentant un traitement de commande avec pools, couloirs et passerelles",
      petriCaption: "Des jetons aux équations. Votre réseau de Petri et ses matrices, côte à côte.", petriAlt: "Réseau de Petri d’exclusion mutuelle avec jetons, transitions rectangulaires et matrices Pre, Post et d’incidence",
      miningCaption: "Des événements aux relations. Suivez l’algorithme Alpha, étape par étape.", miningAlt: "Espace de découverte de processus avec journal d’événements, matrice des relations et algorithme Alpha",
      splitCaption: "De la place pour raisonner. Un BPMN et les matrices de Petri dans des panneaux partagés.", splitAlt: "Espace partagé affichant un modèle BPMN à gauche et les matrices d’un réseau de Petri à droite"
    }
  };

  // English by default, matching the app. Storage is optional (private browsing / file://).
  let language = "en";
  try { if (localStorage.getItem("open-process-site-language") === "fr") language = "fr"; } catch { /* Keep default. */ }
  let view = "bpmn";
  let releaseState = "initial";
  let release = null;
  const views = ["bpmn", "petri", "mining", "split"];
  const tabs = [...document.querySelectorAll("[data-view]")];
  const previewImage = document.getElementById("preview-image");
  const dialog = document.getElementById("preview-dialog");
  const translate = (key) => strings[language][key];

  function updatePreview() {
    previewImage.src = `assets/${view}.webp`;
    previewImage.alt = translate(`${view}Alt`);
    document.getElementById("preview-caption").textContent = translate(`${view}Caption`);
    document.getElementById("preview-number").textContent = `0${views.indexOf(view) + 1} / 04`;
    document.getElementById("preview-panel").setAttribute("aria-labelledby", `tab-${view}`);
    document.getElementById("dialog-image").src = previewImage.src;
    document.getElementById("dialog-image").alt = previewImage.alt;
    document.getElementById("dialog-caption").textContent = translate(`${view}Caption`);
    tabs.forEach((tab) => {
      const selected = tab.dataset.view === view;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
    });
  }

  function safeGithubUrl(value) {
    try {
      const url = new URL(value);
      return url.protocol === "https:" && url.origin === "https://github.com" && url.pathname.startsWith("/linventif/open-process/releases/") ? url.href : null;
    } catch { return null; }
  }

  function updateDownloads() {
    const stateKey = { initial: "releaseInitial", none: "releaseNone", error: "releaseError", available: "releaseAvailable" }[releaseState];
    document.getElementById("release-status").textContent = translate(stateKey).replace("{version}", release?.tag_name || "");
    document.getElementById("release-link").href = safeGithubUrl(release?.html_url) || releasesUrl;
    const assetNames = {
      windows: "open-process-x86_64-pc-windows-msvc.exe",
      linux: "open-process-x86_64-unknown-linux-gnu",
      "mac-arm": "open-process-aarch64-apple-darwin",
      "mac-intel": "open-process-x86_64-apple-darwin"
    };
    document.querySelectorAll(".download-link").forEach((link) => {
      const platform = link.dataset.platform === "mac" ? document.getElementById("mac-architecture").value : link.dataset.platform;
      const asset = release?.assets?.find((entry) => entry.name === assetNames[platform]);
      const downloadUrl = safeGithubUrl(asset?.browser_download_url);
      link.href = downloadUrl || releasesUrl;
      link.querySelector("[data-download-label]").textContent = translate(downloadUrl ? "download" : "viewReleases");
      link.lastElementChild.textContent = downloadUrl ? "↓" : "↗";
    });
  }

  function applyLanguage() {
    document.documentElement.lang = language;
    document.title = translate("title");
    document.querySelector('meta[name="description"]').content = translate("description");
    document.querySelector('meta[property="og:title"]').content = translate("title");
    document.querySelector('meta[property="og:description"]').content = translate("description");
    document.querySelectorAll("[data-i18n]").forEach((element) => { element.textContent = translate(element.dataset.i18n); });
    document.querySelectorAll("[data-i18n-aria]").forEach((element) => { element.setAttribute("aria-label", translate(element.dataset.i18nAria)); });
    document.getElementById("language-label").textContent = language === "en" ? "FR" : "EN";
    const languageToggle = document.getElementById("language-toggle");
    languageToggle.setAttribute("aria-label", language === "en" ? "Passer en français" : "Switch to English");
    languageToggle.lang = language === "en" ? "fr" : "en";
    document.querySelector('#mac-architecture option[value="mac-arm"]').textContent = translate("macArm");
    updatePreview();
    updateDownloads();
  }

  document.getElementById("language-toggle").addEventListener("click", () => {
    language = language === "en" ? "fr" : "en";
    try { localStorage.setItem("open-process-site-language", language); } catch { /* Optional preference. */ }
    applyLanguage();
  });

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => { view = tab.dataset.view; updatePreview(); });
    tab.addEventListener("keydown", (event) => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const current = views.indexOf(view);
      const index = event.key === "Home" ? 0 : event.key === "End" ? views.length - 1 : (current + (event.key === "ArrowRight" ? 1 : -1) + views.length) % views.length;
      view = views[index];
      updatePreview();
      tabs[index].focus();
    });
  });

  document.getElementById("mac-architecture").addEventListener("change", updateDownloads);
  document.getElementById("expand-preview").addEventListener("click", () => {
    if (typeof dialog.showModal === "function") dialog.showModal();
    else window.open(previewImage.src, "_blank", "noopener");
  });
  document.getElementById("close-preview").addEventListener("click", () => dialog.close());
  dialog.addEventListener("click", (event) => { if (event.target === dialog) dialog.close(); });

  applyLanguage();

  // Progressive enhancement: the static release links remain useful without JS/network.
  async function loadRelease() {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);
    try {
      const response = await fetch("https://api.github.com/repos/linventif/open-process/releases/latest", {
        headers: { Accept: "application/vnd.github+json" }, signal: controller.signal
      });
      if (response.status === 404) {
        releaseState = "none";
      } else {
        if (!response.ok) throw new Error("Release unavailable");
        const data = await response.json();
        if (typeof data.tag_name !== "string" || !Array.isArray(data.assets) || !safeGithubUrl(data.html_url)) throw new Error("Invalid release metadata");
        release = data;
        releaseState = "available";
      }
    } catch {
      releaseState = "error";
    } finally {
      clearTimeout(timeout);
      updateDownloads();
    }
  }
  loadRelease();
})();
