import type { Language, LegalPage } from "./messages";
import { publisher, repository } from "./site";

export const legalUpdated = "2026-11-14";
export const legalPages = ["terms", "privacy"] as const satisfies readonly LegalPage[];

interface LegalSection {
  id: string;
  title: string;
  paragraphs: string[];
  links?: { label: string; href: string }[];
}

interface LegalDocument {
  title: string;
  description: string;
  intro: string;
  sections: LegalSection[];
}

interface LegalMessages {
  eyebrow: string;
  updated: string;
  home: string;
  contents: string;
  navigation: string;
  contact: string;
  termsLabel: string;
  privacyLabel: string;
  terms: LegalDocument;
  privacy: LegalDocument;
}

const links = {
  licenses: [
    { label: "MIT", href: `${repository}/blob/main/LICENSE-MIT` },
    { label: "Apache 2.0", href: `${repository}/blob/main/LICENSE-APACHE` },
  ],
  cloudflare: { label: "Cloudflare Privacy Policy", href: "https://www.cloudflare.com/privacypolicy/" },
  github: { label: "GitHub Privacy Statement", href: "https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement" },
};

const en: LegalMessages = {
  eyebrow: "OPEN PROCESS / LEGAL",
  updated: "Last updated",
  home: "Back to home",
  contents: "On this page",
  navigation: "Legal information",
  contact: "Contact",
  termsLabel: "Terms of use",
  privacyLabel: "Privacy policy",
  terms: {
    title: "Terms of use",
    description: "Publisher, open-source licenses and terms for using the Open Process website and application.",
    intro: "Open Process is a free, open-source workspace for diagrams and process modelling. These terms describe the website and the application available from it.",
    sections: [
      {
        id: "publisher", title: "Publisher and contact",
        paragraphs: [`Open Process is maintained by ${publisher.name}, owner of the linventif/open-process repository. For questions about the website or these terms, email ${publisher.email}.`, "The website and browser application are hosted on Cloudflare Pages, provided by Cloudflare, Inc."],
        links: [{ label: publisher.name, href: publisher.url }, { label: "Source repository", href: repository }, { label: "Cloudflare", href: "https://www.cloudflare.com/" }],
      },
      {
        id: "access", title: "Access to the service",
        paragraphs: ["The website and application are available without an account or subscription. Desktop downloads are distributed through GitHub Releases. An internet connection is needed to load the website or download a release.", "Features, compatibility and availability may change as the project develops. Maintenance and support depend on maintainer availability; there is no guaranteed uptime or response time."],
      },
      {
        id: "licenses", title: "Open-source licenses",
        paragraphs: ["Open Process is distributed under your choice of the MIT License or the Apache License, Version 2.0. Those licenses govern use, copying, modification and redistribution of the software. These website terms do not restrict the rights they grant.", "Third-party components, fonts and other assets retain their own licenses and notices. Consult the repository and the notices included with a release."],
        links: links.licenses,
      },
      {
        id: "documents", title: "Your documents and responsibilities",
        paragraphs: ["Your diagrams, imported files and exports remain yours or their respective rights holders’. Editing them does not transfer ownership to the project. You are responsible for having permission to use the content you import or share.", "The application processes documents on your device and saves automatic copies locally. Keep separate .opx backups: clearing browser data, private browsing, storage limits or device failure may remove local copies. Automatic saving is not an online backup service.", "Use the website lawfully and respect other people’s rights. Do not interfere with its availability or security. Use anonymised examples when posting public support requests."],
      },
      {
        id: "results", title: "Results, warranties and liability",
        paragraphs: ["Models, conversions, generated SQL and analysis results can contain errors or have documented limits. Review them before relying on them in production, teaching, research or other decisions.", "The software is supplied as is, with the warranty disclaimers and liability limitations set out in the applicable open-source license, to the extent permitted by law. These terms do not exclude rights or liabilities that applicable law does not allow to be excluded."],
      },
      {
        id: "external", title: "External services and privacy",
        paragraphs: ["GitHub hosts the source code, public discussions and release downloads. External websites and services apply their own terms and privacy policies. The privacy policy explains local storage and requests made to hosting and download services."],
        links: [{ label: "Privacy policy", href: "/privacy/" }, { label: "Project support", href: `${repository}/blob/main/SUPPORT.md` }],
      },
      {
        id: "changes", title: "Changes to these terms",
        paragraphs: [`Changes will be published on this page with an updated date. For questions, contact ${publisher.email}. The software license accompanying your copy continues to govern that copy.`],
      },
    ],
  },
  privacy: {
    title: "Privacy policy",
    description: "How Open Process handles local documents, browser storage, hosting requests and contact messages.",
    intro: "Your models are processed on your device. This page explains what stays locally, what web requests reach external services and how to contact us about personal data.",
    sections: [
      {
        id: "contact", title: "Who to contact",
        paragraphs: [`${publisher.name} maintains Open Process through the linventif/open-process repository and is the contact responsible for this website. For privacy questions or requests concerning personal data handled by the project, email ${publisher.email}.`],
        links: [{ label: publisher.name, href: publisher.url }],
      },
      {
        id: "documents", title: "Documents stay on your device",
        paragraphs: ["The application processes diagrams, event logs, imported files and analysis results locally. Its editing and saving features do not upload your documents to an application server, and there is no account-based document synchronisation.", "In a browser, local storage holds automatic document copies, the restored workspace, preferences such as language and cached previews. On desktop, automatic copies are saved in Documents/Open Process and session settings in the application’s local storage. Manual saves and exports create separate files."],
      },
      {
        id: "storage", title: "Local storage, cookies and deletion",
        paragraphs: ["Local storage is used to save and restore your work and preferences. The site and app do not include advertising cookies or an analytics or telemetry SDK. Fonts and visual assets are served with the site.", "Browser copies remain until removed by you or the browser; they are not subject to a scheduled expiry. To clear them, first export any .opx files you want to keep, close the app’s tabs, then clear site data for open-process.linv.dev in your browser settings. This also removes saved preferences and previews.", "For desktop data, quit the app before removing the automatic library in Documents/Open Process and its saved application settings. Manually saved files, exports and downloaded backups must be deleted separately. Closing a document does not delete its automatic copy."],
      },
      {
        id: "hosting", title: "Hosting and technical requests",
        paragraphs: ["Cloudflare Pages serves the website and web app. Cloudflare may process your IP address, request URL, time and browser information to deliver and protect the service. Its network may process data outside your country.", "Hosting log retention and optional hosting analytics depend on the production configuration. No fixed retention period or absence of provider logs is claimed here; contact us for information about the current settings."],
        links: [links.cloudflare],
      },
      {
        id: "github", title: "GitHub and public contributions",
        paragraphs: ["The home page requests public release metadata from api.github.com to show current downloads. GitHub receives ordinary connection data, including your IP address; diagram content is not sent. Visiting repository links or downloading releases also contacts GitHub.", "Issues, pull requests and attachments you submit on GitHub are public. Do not include confidential models, credentials or personal information in them. GitHub’s own privacy and international transfer terms apply to its services."],
        links: [links.github],
      },
      {
        id: "messages", title: "Contact messages and purposes",
        paragraphs: ["If you email us, we receive your email address and any name, message and attachments you choose to provide. The maintainer and the email service involved in delivering the exchange handle this information to answer your request, maintain the project or address a security report. Only send information needed for the request.", "For contact handling and service security, the project relies on its legitimate interests in responding to requests and operating a secure service. A legal obligation may require particular records to be kept. Correspondence is retained for as long as needed to resolve the request and meet any applicable legal obligations; a sensitive security report may need follow-up after the initial answer."],
      },
      {
        id: "rights", title: "Your rights",
        paragraphs: [`Where applicable data protection law provides them, you may request access, correction, erasure, restriction or portability of personal data held by the project, or object to processing based on legitimate interests. Write to ${publisher.email}, describing your request. These rights depend on the data and the legal basis involved.`, "The maintainer cannot retrieve or remotely erase documents stored only on your device. Use the deletion instructions above for those copies. You may also complain to the data protection authority where you live or work; in France, this is the CNIL."],
        links: [{ label: "CNIL — Your rights", href: "https://www.cnil.fr/en/understand-your-rights" }],
      },
      {
        id: "changes", title: "Updates to this policy",
        paragraphs: ["This page will be updated when the described practices change. Its last updated date appears above. Please contact us if you need clarification about the current hosting configuration or a particular use of your data."],
      },
    ],
  },
};

const fr: LegalMessages = {
  eyebrow: "OPEN PROCESS / INFORMATIONS LÉGALES",
  updated: "Dernière mise à jour",
  home: "Retour à l’accueil",
  contents: "Sur cette page",
  navigation: "Informations légales",
  contact: "Contact",
  termsLabel: "Conditions d’utilisation",
  privacyLabel: "Confidentialité",
  terms: {
    title: "Conditions d’utilisation",
    description: "Éditeur, licences libres et conditions d’utilisation du site et de l’application Open Process.",
    intro: "Open Process est un espace de travail gratuit et libre pour les diagrammes et la modélisation de processus. Ces conditions présentent le site et l’application qui y est proposée.",
    sections: [
      {
        id: "publisher", title: "Éditeur et contact",
        paragraphs: [`Open Process est maintenu par ${publisher.name}, propriétaire du dépôt linventif/open-process. Pour toute question sur le site ou ces conditions, écrivez à ${publisher.email}.`, "Le site et l’application web sont hébergés sur Cloudflare Pages, un service de Cloudflare, Inc."],
        links: [{ label: publisher.name, href: publisher.url }, { label: "Dépôt source", href: repository }, { label: "Cloudflare", href: "https://www.cloudflare.com/" }],
      },
      {
        id: "access", title: "Accès au service",
        paragraphs: ["Le site et l’application sont accessibles sans compte ni abonnement. Les versions de bureau sont distribuées via GitHub Releases. Une connexion Internet est nécessaire pour charger le site ou télécharger une version.", "Les fonctionnalités, la compatibilité et la disponibilité peuvent évoluer avec le projet. La maintenance et l’assistance dépendent de la disponibilité des responsables ; aucun délai de réponse ni taux de disponibilité n’est garanti."],
      },
      {
        id: "licenses", title: "Licences libres",
        paragraphs: ["Open Process est distribué, au choix, sous licence MIT ou Apache, version 2.0. Ces licences régissent l’utilisation, la copie, la modification et la redistribution du logiciel. Les présentes conditions du site ne limitent pas les droits qu’elles accordent.", "Les composants tiers, polices et autres ressources conservent leurs propres licences et mentions. Consultez le dépôt et les mentions accompagnant chaque version."],
        links: links.licenses,
      },
      {
        id: "documents", title: "Vos documents et responsabilités",
        paragraphs: ["Vos diagrammes, fichiers importés et exports restent votre propriété ou celle de leurs ayants droit. Leur édition ne transfère aucun droit de propriété au projet. Vous devez disposer des autorisations nécessaires pour utiliser les contenus importés ou partagés.", "L’application traite vos documents sur votre appareil et conserve ses copies automatiques localement. Gardez des sauvegardes .opx séparées : l’effacement des données du navigateur, la navigation privée, les limites de stockage ou une panne peuvent supprimer les copies locales. La sauvegarde automatique n’est pas un service de sauvegarde en ligne.", "Utilisez le site légalement et respectez les droits des tiers. Ne perturbez pas sa disponibilité ni sa sécurité. Utilisez des exemples anonymisés pour les demandes d’aide publiques."],
      },
      {
        id: "results", title: "Résultats, garanties et responsabilité",
        paragraphs: ["Les modèles, conversions, requêtes SQL générées et résultats d’analyse peuvent contenir des erreurs ou présenter des limites documentées. Vérifiez-les avant de les utiliser en production, en enseignement, en recherche ou pour prendre une décision.", "Le logiciel est fourni en l’état, avec les exclusions de garantie et limitations de responsabilité prévues par la licence libre applicable, dans les limites autorisées par la loi. Ces conditions n’excluent aucun droit ni aucune responsabilité que la loi applicable interdit d’exclure."],
      },
      {
        id: "external", title: "Services externes et confidentialité",
        paragraphs: ["GitHub héberge le code source, les discussions publiques et les téléchargements. Les sites et services externes appliquent leurs propres conditions et politiques de confidentialité. La politique de confidentialité explique le stockage local et les requêtes adressées aux services d’hébergement et de téléchargement."],
        links: [{ label: "Politique de confidentialité", href: "/fr/privacy/" }, { label: "Assistance du projet", href: `${repository}/blob/main/SUPPORT.md` }],
      },
      {
        id: "changes", title: "Modification des conditions",
        paragraphs: [`Les changements seront publiés sur cette page avec une date actualisée. Pour toute question, contactez ${publisher.email}. La licence accompagnant votre copie du logiciel continue de régir cette copie.`],
      },
    ],
  },
  privacy: {
    title: "Politique de confidentialité",
    description: "Comment Open Process gère les documents locaux, le stockage du navigateur, l’hébergement et les messages de contact.",
    intro: "Vos modèles sont traités sur votre appareil. Cette page explique ce qui reste local, les requêtes web adressées à des services externes et comment nous contacter au sujet des données personnelles.",
    sections: [
      {
        id: "contact", title: "Votre interlocuteur",
        paragraphs: [`${publisher.name} maintient Open Process via le dépôt linventif/open-process et est l’interlocuteur responsable de ce site. Pour toute question de confidentialité ou demande relative aux données personnelles traitées par le projet, écrivez à ${publisher.email}.`],
        links: [{ label: publisher.name, href: publisher.url }],
      },
      {
        id: "documents", title: "Les documents restent sur votre appareil",
        paragraphs: ["L’application traite localement les diagrammes, journaux d’événements, fichiers importés et résultats d’analyse. Ses fonctions d’édition et de sauvegarde n’envoient pas vos documents à un serveur applicatif. Il n’existe pas de synchronisation de documents associée à un compte.", "Dans le navigateur, le stockage local conserve les copies automatiques, l’espace de travail restauré, les préférences dont la langue et les aperçus en cache. Sur ordinateur, les copies automatiques sont enregistrées dans Documents/Open Process et les paramètres de session dans le stockage local de l’application. Les sauvegardes manuelles et exports créent des fichiers séparés."],
      },
      {
        id: "storage", title: "Stockage local, cookies et suppression",
        paragraphs: ["Le stockage local sert à enregistrer et restaurer votre travail et vos préférences. Le site et l’application n’intègrent ni cookies publicitaires ni SDK de mesure d’audience ou de télémétrie. Les polices et ressources visuelles sont servies avec le site.", "Les copies du navigateur restent présentes jusqu’à leur suppression par vous ou par le navigateur, sans expiration programmée. Pour les effacer, exportez d’abord les fichiers .opx à conserver, fermez les onglets de l’application, puis effacez les données du site open-process.linv.dev dans les paramètres du navigateur. Cela supprime aussi les préférences et aperçus enregistrés.", "Sur ordinateur, quittez l’application avant de supprimer la bibliothèque automatique dans Documents/Open Process et les paramètres enregistrés de l’application. Les fichiers sauvegardés manuellement, exports et copies téléchargées doivent être supprimés séparément. Fermer un document ne supprime pas sa copie automatique."],
      },
      {
        id: "hosting", title: "Hébergement et requêtes techniques",
        paragraphs: ["Cloudflare Pages diffuse le site et l’application web. Cloudflare peut traiter votre adresse IP, l’URL demandée, l’heure et les informations du navigateur pour fournir et protéger le service. Son réseau peut traiter des données hors de votre pays.", "La conservation des journaux et les éventuelles statistiques d’hébergement dépendent de la configuration de production. Cette page n’affirme ni durée fixe de conservation ni absence de journaux chez l’hébergeur ; contactez-nous pour connaître les réglages actuels."],
        links: [{ ...links.cloudflare, label: "Politique de confidentialité de Cloudflare" }],
      },
      {
        id: "github", title: "GitHub et contributions publiques",
        paragraphs: ["La page d’accueil demande les métadonnées publiques des versions à api.github.com pour afficher les téléchargements disponibles. GitHub reçoit les données de connexion habituelles, dont votre adresse IP ; le contenu des diagrammes n’est pas envoyé. Les liens vers le dépôt et les téléchargements contactent aussi GitHub.", "Les issues, demandes de fusion et pièces jointes que vous publiez sur GitHub sont publiques. N’y incluez ni modèles confidentiels, ni identifiants secrets, ni données personnelles. Les règles de confidentialité et de transferts internationaux de GitHub s’appliquent à ses services."],
        links: [{ ...links.github, label: "Politique de confidentialité de GitHub" }],
      },
      {
        id: "messages", title: "Messages de contact et finalités",
        paragraphs: ["Si vous nous écrivez, nous recevons votre adresse électronique et les nom, message et pièces jointes que vous choisissez de transmettre. Le responsable du projet et le service de messagerie acheminant l’échange traitent ces informations pour répondre, maintenir le projet ou examiner un signalement de sécurité. N’envoyez que les informations nécessaires à votre demande.", "Pour les échanges et la sécurité du service, le projet s’appuie sur son intérêt légitime à répondre aux demandes et à assurer un service sûr. Une obligation légale peut imposer de conserver certains éléments. Les échanges sont conservés le temps nécessaire au traitement de la demande et au respect des obligations légales applicables ; un signalement de sécurité sensible peut nécessiter un suivi après la première réponse."],
      },
      {
        id: "rights", title: "Vos droits",
        paragraphs: [`Lorsque la réglementation applicable les prévoit, vous pouvez demander l’accès, la rectification, l’effacement, la limitation ou la portabilité des données personnelles détenues par le projet, ou vous opposer à un traitement fondé sur l’intérêt légitime. Écrivez à ${publisher.email} en précisant votre demande. Ces droits dépendent des données et de la base légale concernées.`, "Le responsable du projet ne peut ni récupérer ni effacer à distance les documents présents uniquement sur votre appareil. Utilisez les instructions de suppression ci-dessus. Vous pouvez aussi saisir l’autorité de protection des données de votre lieu de résidence ou de travail ; en France, il s’agit de la CNIL."],
        links: [{ label: "CNIL — Comprendre vos droits", href: "https://www.cnil.fr/fr/comprendre-vos-droits" }],
      },
      {
        id: "changes", title: "Mise à jour de cette politique",
        paragraphs: ["Cette page sera mise à jour lorsque les pratiques décrites changeront. Sa date de mise à jour figure en tête de page. Contactez-nous pour toute précision sur la configuration actuelle de l’hébergement ou une utilisation particulière de vos données."],
      },
    ],
  },
};

const es: LegalMessages = {
  eyebrow: "OPEN PROCESS / INFORMACIÓN LEGAL",
  updated: "Última actualización",
  home: "Volver al inicio",
  contents: "En esta página",
  navigation: "Información legal",
  contact: "Contacto",
  termsLabel: "Condiciones de uso",
  privacyLabel: "Privacidad",
  terms: {
    title: "Condiciones de uso",
    description: "Responsable, licencias de código abierto y condiciones de uso del sitio y la aplicación Open Process.",
    intro: "Open Process es un espacio de trabajo gratuito y de código abierto para diagramas y modelado de procesos. Estas condiciones describen el sitio y la aplicación que ofrece.",
    sections: [
      {
        id: "publisher", title: "Responsable y contacto",
        paragraphs: [`Open Process está mantenido por ${publisher.name}, propietario del repositorio linventif/open-process. Para preguntas sobre el sitio o estas condiciones, escribe a ${publisher.email}.`, "El sitio y la aplicación web están alojados en Cloudflare Pages, un servicio de Cloudflare, Inc."],
        links: [{ label: publisher.name, href: publisher.url }, { label: "Repositorio de código", href: repository }, { label: "Cloudflare", href: "https://www.cloudflare.com/" }],
      },
      {
        id: "access", title: "Acceso al servicio",
        paragraphs: ["El sitio y la aplicación están disponibles sin cuenta ni suscripción. Las versiones de escritorio se distribuyen mediante GitHub Releases. Se necesita conexión a Internet para cargar el sitio o descargar una versión.", "Las funciones, la compatibilidad y la disponibilidad pueden cambiar con el desarrollo del proyecto. El mantenimiento y la asistencia dependen de la disponibilidad de sus responsables; no se garantiza un tiempo de respuesta ni de disponibilidad."],
      },
      {
        id: "licenses", title: "Licencias de código abierto",
        paragraphs: ["Open Process se distribuye, a tu elección, bajo la licencia MIT o Apache, versión 2.0. Estas licencias rigen el uso, la copia, la modificación y la redistribución del software. Las presentes condiciones del sitio no restringen los derechos que conceden.", "Los componentes de terceros, fuentes y otros recursos conservan sus propias licencias y avisos. Consulta el repositorio y los avisos incluidos con cada versión."],
        links: links.licenses,
      },
      {
        id: "documents", title: "Tus documentos y responsabilidades",
        paragraphs: ["Tus diagramas, archivos importados y exportaciones siguen siendo tuyos o de sus respectivos titulares. Editarlos no transfiere su propiedad al proyecto. Debes tener permiso para utilizar el contenido que importas o compartes.", "La aplicación procesa los documentos en tu dispositivo y guarda copias automáticas localmente. Conserva copias .opx separadas: borrar los datos del navegador, la navegación privada, los límites de almacenamiento o un fallo del dispositivo pueden eliminar las copias locales. El guardado automático no es un servicio de copias de seguridad en línea.", "Usa el sitio de forma legal y respeta los derechos de otras personas. No interfieras con su disponibilidad o seguridad. Utiliza ejemplos anonimizados en las solicitudes públicas de ayuda."],
      },
      {
        id: "results", title: "Resultados, garantías y responsabilidad",
        paragraphs: ["Los modelos, conversiones, SQL generado y resultados de análisis pueden contener errores o tener límites documentados. Revísalos antes de utilizarlos en producción, docencia, investigación u otras decisiones.", "El software se ofrece tal cual, con las exclusiones de garantía y limitaciones de responsabilidad de la licencia de código abierto aplicable, en la medida permitida por la ley. Estas condiciones no excluyen derechos ni responsabilidades que la legislación aplicable no permita excluir."],
      },
      {
        id: "external", title: "Servicios externos y privacidad",
        paragraphs: ["GitHub aloja el código fuente, las conversaciones públicas y las descargas. Los sitios y servicios externos aplican sus propias condiciones y políticas de privacidad. La política de privacidad explica el almacenamiento local y las solicitudes a los servicios de alojamiento y descarga."],
        links: [{ label: "Política de privacidad", href: "/es/privacy/" }, { label: "Asistencia del proyecto", href: `${repository}/blob/main/SUPPORT.md` }],
      },
      {
        id: "changes", title: "Cambios en estas condiciones",
        paragraphs: [`Los cambios se publicarán en esta página con una fecha actualizada. Para cualquier pregunta, escribe a ${publisher.email}. La licencia que acompaña a tu copia del software sigue rigiendo esa copia.`],
      },
    ],
  },
  privacy: {
    title: "Política de privacidad",
    description: "Cómo gestiona Open Process los documentos locales, el almacenamiento del navegador, el alojamiento y los mensajes de contacto.",
    intro: "Tus modelos se procesan en tu dispositivo. Esta página explica qué permanece localmente, qué solicitudes web llegan a servicios externos y cómo contactarnos sobre datos personales.",
    sections: [
      {
        id: "contact", title: "A quién contactar",
        paragraphs: [`${publisher.name} mantiene Open Process mediante el repositorio linventif/open-process y es el contacto responsable de este sitio. Para consultas de privacidad o solicitudes sobre datos personales tratados por el proyecto, escribe a ${publisher.email}.`],
        links: [{ label: publisher.name, href: publisher.url }],
      },
      {
        id: "documents", title: "Los documentos permanecen en tu dispositivo",
        paragraphs: ["La aplicación procesa localmente diagramas, registros de eventos, archivos importados y resultados de análisis. Sus funciones de edición y guardado no suben tus documentos a un servidor de aplicaciones y no existe sincronización de documentos vinculada a una cuenta.", "En el navegador, el almacenamiento local conserva copias automáticas, el espacio de trabajo restaurado, preferencias como el idioma y vistas previas en caché. En el escritorio, las copias automáticas se guardan en Documentos/Open Process y los ajustes de sesión en el almacenamiento local de la aplicación. El guardado manual y las exportaciones crean archivos separados."],
      },
      {
        id: "storage", title: "Almacenamiento local, cookies y eliminación",
        paragraphs: ["El almacenamiento local permite guardar y restaurar tu trabajo y preferencias. El sitio y la aplicación no incluyen cookies publicitarias ni un SDK de analítica o telemetría. Las fuentes y los recursos visuales se sirven con el sitio.", "Las copias del navegador permanecen hasta que tú o el navegador las eliminéis; no tienen una caducidad programada. Para borrarlas, exporta primero los archivos .opx que quieras conservar, cierra las pestañas de la aplicación y borra los datos del sitio open-process.linv.dev en los ajustes del navegador. Esto también elimina preferencias y vistas previas guardadas.", "En el escritorio, cierra la aplicación antes de eliminar la biblioteca automática en Documentos/Open Process y los ajustes guardados de la aplicación. Los archivos guardados manualmente, exportaciones y copias descargadas deben eliminarse por separado. Cerrar un documento no elimina su copia automática."],
      },
      {
        id: "hosting", title: "Alojamiento y solicitudes técnicas",
        paragraphs: ["Cloudflare Pages sirve el sitio y la aplicación web. Cloudflare puede tratar tu dirección IP, la URL solicitada, la hora y datos del navegador para prestar y proteger el servicio. Su red puede tratar datos fuera de tu país.", "La conservación de registros y las posibles estadísticas del alojamiento dependen de la configuración de producción. Aquí no se afirma un plazo fijo de conservación ni la ausencia de registros del proveedor; contáctanos para conocer los ajustes actuales."],
        links: [{ ...links.cloudflare, label: "Política de privacidad de Cloudflare" }],
      },
      {
        id: "github", title: "GitHub y contribuciones públicas",
        paragraphs: ["La página de inicio solicita metadatos públicos de versiones a api.github.com para mostrar las descargas actuales. GitHub recibe datos habituales de conexión, incluida tu dirección IP; no se envía el contenido de los diagramas. Los enlaces al repositorio y las descargas también contactan con GitHub.", "Las incidencias, solicitudes de incorporación y archivos adjuntos que publiques en GitHub son públicos. No incluyas modelos confidenciales, credenciales ni datos personales. Sus servicios están sujetos a las condiciones de privacidad y transferencias internacionales de GitHub."],
        links: [{ ...links.github, label: "Política de privacidad de GitHub" }],
      },
      {
        id: "messages", title: "Mensajes de contacto y finalidades",
        paragraphs: ["Si nos escribes, recibimos tu dirección de correo y cualquier nombre, mensaje y archivo adjunto que decidas enviar. El responsable del proyecto y el servicio de correo que entrega el intercambio tratan esta información para responder, mantener el proyecto o atender un informe de seguridad. Envía solo la información necesaria para tu solicitud.", "Para gestionar consultas y proteger el servicio, el proyecto se basa en su interés legítimo en responder y mantener un servicio seguro. Una obligación legal puede exigir conservar determinados registros. La correspondencia se conserva mientras sea necesaria para resolver la solicitud y cumplir las obligaciones legales aplicables; un informe de seguridad sensible puede requerir seguimiento tras la primera respuesta."],
      },
      {
        id: "rights", title: "Tus derechos",
        paragraphs: [`Cuando la legislación aplicable los reconozca, puedes solicitar acceso, rectificación, supresión, limitación o portabilidad de los datos personales que tenga el proyecto, u oponerte al tratamiento basado en intereses legítimos. Escribe a ${publisher.email} describiendo tu solicitud. Estos derechos dependen de los datos y de la base jurídica del tratamiento.`, "El responsable no puede recuperar ni borrar a distancia documentos almacenados únicamente en tu dispositivo. Utiliza las instrucciones anteriores para eliminar esas copias. También puedes reclamar ante la autoridad de protección de datos de tu lugar de residencia o trabajo; en Francia, es la CNIL."],
        links: [{ label: "CNIL — Tus derechos", href: "https://www.cnil.fr/en/understand-your-rights" }],
      },
      {
        id: "changes", title: "Actualizaciones de esta política",
        paragraphs: ["Esta página se actualizará cuando cambien las prácticas descritas. La fecha de actualización aparece al principio. Contáctanos si necesitas aclaraciones sobre la configuración actual del alojamiento o un uso particular de tus datos."],
      },
    ],
  },
};

export const legalMessages = { en, fr, es } satisfies Record<Language, LegalMessages>;
