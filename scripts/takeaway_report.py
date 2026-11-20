#!/usr/bin/env python3
"""Build the French TakeAway report and submission archive from validated outputs.

The application owns discovery, BPMN conversion and sales analysis. This optional
publication tool assembles their editable outputs, independently calculated
figures and recorded checks into an academic PDF/HTML deliverable. It never
rewrites source CSV/XLSX files or invents missing identities, dates or currencies.
"""

from __future__ import annotations

import argparse
import csv
from collections import Counter, defaultdict
from datetime import date
from decimal import Decimal, ROUND_HALF_UP
import hashlib
import html
import io
import json
from pathlib import Path
import re
import shutil
import subprocess
import zipfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
CHANNELS = ("Counter", "Phone", "Internet")
CHANNEL_FR = {"Counter": "Comptoir", "Phone": "Téléphone", "Internet": "Internet"}
WEEKDAYS = ("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")
DAY_FR = dict(zip(WEEKDAYS, ("Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche")))
MARKER = re.compile(r"(?:vide|empty)(?:[\s_-]*\d+)?", re.IGNORECASE)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def native_fingerprint(path: Path) -> str:
    value = 0xcbf29ce484222325
    for byte in path.read_bytes():
        value = ((value ^ byte) * 0x100000001b3) & ((1 << 64)-1)
    return f"fnv1a64:{value:016x}"


def fr_number(value, places=2) -> str:
    if value is None:
        return "Non défini"
    number = Decimal(str(value)).quantize(Decimal(1).scaleb(-places), rounding=ROUND_HALF_UP)
    return f"{number:,.{places}f}".replace(",", "\u202f").replace(".", ",")


def percent(value) -> str:
    return fr_number(Decimal(str(value)) * 100) + " %" if value is not None else "Non défini"


def label(row, key):
    return row.get(key, row.get("dimensions", {}).get(key, ""))


def event_facts(tp: Path) -> dict:
    facts = {}
    for channel in CHANNELS:
        path = tp / "clean" / f"cleanTakeAway{channel}.csv"
        with path.open(encoding="utf-8-sig", newline="") as source:
            rows = list(csv.DictReader(source, delimiter=";"))
        cases = defaultdict(list)
        actors = defaultdict(Counter)
        missing = []
        for row in rows:
            cases[row["CaseId"]].append(row["Activity"])
            if not MARKER.fullmatch(row["Activity"].strip()):
                actor = row.get("Actor", "").strip()
                actors[row["Activity"]][actor or "Non renseigné"] += 1
                if not actor:
                    missing.append({"case_id": row["CaseId"], "activity": row["Activity"]})
        variants = Counter(tuple(trace) for trace in cases.values())
        loops = Counter(trace.count("Prepare") for trace in cases.values() if "Prepare" in trace)
        cash_first = sum(trace.index("Cash") < trace.index("Give order") for trace in cases.values() if "Cash" in trace)
        handover_first = sum(trace.index("Give order") < trace.index("Cash") for trace in cases.values() if "Cash" in trace)
        facts[channel] = {
            "events": len(rows), "cases": len(cases), "variants": len(variants),
            "preparation_cycles": dict(sorted(loops.items())),
            "delivered_cases": sum("Deliver" in trace for trace in cases.values()),
            "contact_cases": sum("Contact customer" in trace for trace in cases.values()),
            "modified_cases": sum("Modify order" in trace for trace in cases.values()),
            "cancelled_cases": sum("Cancel order" in trace for trace in cases.values()),
            "cash_first": cash_first, "handover_first": handover_first,
            "actors": {activity: dict(counts) for activity, counts in sorted(actors.items())},
            "missing_actor_events": missing, "source": str(path.relative_to(ROOT)), "sha256": sha256(path),
        }
    if sum(row["cases"] for row in facts.values()) != 327:
        raise ValueError("The case count changed; update and review the TP narrative before publishing")
    return facts


def model_files(models: Path):
    result = {}
    for channel in (*CHANNELS, "Global"):
        stem = f"takeaway-{channel.lower()}"
        paths = {suffix: models / f"{stem}.{suffix}" for suffix in ("opx", "bpmn", "svg")}
        for path in paths.values():
            if not path.is_file():
                raise FileNotFoundError(f"Generate the Open Process models first: {path}")
        native = load_json(paths["opx"])
        if native["document"]["kind"] != "bpmn":
            raise ValueError(paths["opx"])
        diagram = native["document"]["model"]
        ET.fromstring(paths["bpmn"].read_bytes())
        ET.fromstring(paths["svg"].read_bytes())
        ids = {node["id"] for node in diagram["nodes"]}
        if len(ids) != len(diagram["nodes"]) or any(flow["from"] not in ids or flow["to"] not in ids for flow in diagram["flows"]):
            raise ValueError(f"Invalid graph identities/endpoints: {paths['opx']}")
        tasks = [node for node in diagram["nodes"] if isinstance(node["kind"], dict) and "Task" in node["kind"]]
        if any(not node["name"].strip() or MARKER.fullmatch(node["name"].strip()) for node in tasks):
            raise ValueError(f"Routing placeholders must not appear as business tasks: {channel}")
        result[channel] = {"paths": paths, "diagram": diagram, "tasks": tasks}
    return result


def paragraph(text):
    return {"kind": "p", "text": text}


def heading(text, level=1):
    return {"kind": "heading", "text": text, "level": level}


def table(headers, rows, small=False):
    return {"kind": "table", "headers": headers, "rows": [[str(cell) for cell in row] for row in rows], "small": small}


def bullets(*items):
    return {"kind": "bullets", "items": list(items)}


def figure(path, caption):
    return {"kind": "figure", "path": str(path), "caption": caption}


def page():
    return {"kind": "page"}


def sections(summary, facts, models, validation, verification, analysis_root, xsd):
    totals = summary["totals"]
    groups = summary["tables"]
    figures = {entry["id"]: entry for entry in summary["figures"]}

    def plot(prefix):
        matches = [item for key, item in figures.items() if key.startswith(prefix)]
        if len(matches) != 1:
            raise ValueError(f"Expected one figure for {prefix}, got {list(figures)}")
        entry = matches[0]
        return figure(analysis_root / entry["png"], entry["caption_fr"])

    def metric_rows(key, dimension):
        return [[label(row, dimension), row["recorded_sales"], fr_number(row["revenue"]), fr_number(row["cost"]),
                 fr_number(row["profit"]), percent(row["profit_rate"])] for row in groups[key]]

    blocks = [
        heading("Process Intelligence : pizzeria TakeAway", 0),
        paragraph("Découverte des processus, modèles BPMN et analyse quantitative — M2 MIAGe IPM FA, année 2026/2027."),
        paragraph("Rapport final réalisé avec Open Process. Les fichiers sources fournis sont conservés ; les modèles restent modifiables dans le logiciel."),
        heading("Synthèse", 1),
        paragraph("L’étude distingue deux unités : 327 cas de commande observés dans les journaux événementiels et 463 lignes de vente dans les feuilles Sales. Aucune clé commune validée ne permet de confondre ces populations ni d’identifier des clients uniques."),
        table(["Indicateur descriptif", "Résultat"], [
            ["Cas événementiels / canaux", "327 / 3"], ["Ventes enregistrées", totals["recorded_sales"]],
            ["Chiffre d’affaires enregistré", fr_number(totals["revenue"]) + " u.m."],
            ["Coûts enregistrés", fr_number(totals["cost"]) + " u.m."],
            ["Bénéfice calculé (CA − coûts)", fr_number(totals["profit"]) + " u.m."],
            ["KPI 1 — part du bénéfice dans le CA", percent(totals["profit_rate"])],
            ["Satisfaction moyenne / observations", fr_number(totals["satisfaction_average"]) + f" / {totals['satisfaction_observed']}"],
            ["KPI 2 — satisfaction renseignée", percent(Decimal(totals["satisfaction_observed"]) / Decimal(totals["recorded_sales"]))],
        ]),
        paragraph("Le canal Internet concentre le volume de l’échantillon. Les profils de semaine et les segments de clientèle sont descriptifs : les fichiers ne donnent ni dates complètes, ni identifiant de personne, ni unité monétaire, ni barème documenté pour la satisfaction. Les propositions de pilotage restent conditionnelles à ces limites."),
        table(["Demande du sujet", "Réponse fournie"], [
            ["Découvrir les trois modes", "Trois BPMN expliqués, issus des journaux corrigés"],
            ["Intégrer les modes", "Un BPMN global connecté, choix du canal et fin commune"],
            ["Organisation et information", "Couloirs, affectations observées, objets et magasins de données documentés"],
            ["Quantifier l’activité", "Comptages, finances, profils hebdomadaires, géographie, satisfaction et palmarès"],
            ["Proposer deux KPI", "Part du bénéfice dans le CA et taux de satisfaction renseignée"],
            ["Restituer et contrôler", "Rapport PDF/HTML, modèles OPX/BPMN/SVG, CSV et contrôles reproductibles"],
        ]), page(),
        heading("1. Sources, périmètre et méthode"),
        paragraph("Le sujet demande initialement ProM et BizAgi. À la demande du commanditaire de ce travail, les opérations de découverte et de modélisation ont été réalisées dans Open Process, avec les fonctions nécessaires ajoutées au logiciel. Python sert à l’analyse indépendante, aux figures et à la publication du rapport. Aucun résultat n’est présenté comme une sortie de ProM ou de BizAgi."),
        paragraph("Les CSV du répertoire clean constituent la référence comportementale. Les feuilles événementielles des classeurs Counter et Phone sont antérieures à ces corrections ; les classeurs restent utilisables pour leurs feuilles Sales, CaseStructure et PizzasInfo. Les sources originales ne sont pas réparées silencieusement."),
        table(["Canal", "Événements", "Cas", "Variantes brutes", "Ventes"], [
            [CHANNEL_FR[channel], facts[channel]["events"], facts[channel]["cases"], facts[channel]["variants"],
             next(row["recorded_sales"] for row in groups["channels"] if label(row, "channel") == channel)]
            for channel in CHANNELS
        ]),
        bullets(
            "Le CSV garde l’ordre des événements de chaque CaseId et ses colonnes renseignées. Les colonnes entièrement vides en fin de tableau sont seulement retirées de la copie de travail si nécessaire.",
            "La découverte utilise Alpha avec frontières communes pour les traces dont les activités finales peuvent apparaître dans les deux ordres. Les noms de frontière sont ensuite silencieux.",
            "Les marqueurs entiers Vide/Empty et leurs suffixes numériques deviennent des transitions silencieuses après la construction d’Alpha. Les enlever avant l’algorithme changerait les boucles et les choix observés.",
            "La conversion Petri vers BPMN produit des passerelles de routage et des tâches métier nommées. Le retrait d’un connecteur transparent reste conservateur afin de préserver les branches et les boucles.",
            "Le rejeu prouve l’acceptation des traces testées. Il ne démontre ni l’exhaustivité du journal, ni l’équivalence complète des langages, ni la validité de toutes les exécutions futures.",
        ),
        heading("1.1 Qualité et limites des données", 2),
        bullets(
            "Counter : cinq événements Prepare n’ont pas d’acteur renseigné, dans les cas 2005, 2007, 2019, 2025 et 2028. Les acteurs manquants des marqueurs de routage ne sont pas interprétés comme des postes vacants.",
            "Phone : le CSV corrigé emploie Prepare là où l’ancien journal et certains gabarits du classeur emploient Start. La correspondance est explicitement documentée pour comparer les gabarits ; elle ne modifie pas les classeurs.",
            "Sales.Id est un identifiant de ligne local. StructureId référence un gabarit CaseStructure dans le même classeur ; cette référence ne constitue pas une jointure vers les CaseId du CSV, un panier ou une personne.",
            "Les répétitions Prepare–Bake–Pack sont observées, mais leur cause n’est pas donnée. Elles ne prouvent ni défaut de qualité, ni reprise, ni correspondance un cycle = une pizza ou une vente.",
            "Les montants sont additionnés en centièmes entiers. Le bénéfice n’intègre que les coûts fournis, sans inventer salaires, charges, commissions, taxes ou coûts de livraison.",
            "Les résultats mis en cache des formules Get_Revenue/Get_Cost sont lus dans les classeurs ; aucune fonction Excel n’est exécutée ni recalculée par l’importeur.",
            "Les deux scores zéro sont conservés ; trois scores absents restent absents. La moyenne n’est jamais calculée en remplaçant une absence par zéro.",
        ), page(),
        heading("2. Processus découverts par canal"),
        paragraph("Les tâches conservent les libellés anglais du journal pour permettre le rapprochement avec les traces. Le parallélisme du modèle exprime une absence de précédence imposée : il ne prouve pas deux actions physiques simultanées, notamment lorsqu’un même employé réalise encaissement et remise."),
    ]
    descriptions = {
        "Counter": [
            "La prise de commande au comptoir est suivie de Prepare, Bake et Pack. La production peut revenir à Prepare avant de poursuivre vers le règlement et la remise. Aucun événement de livraison n’est enregistré pour ce canal.",
            f"Les {facts['Counter']['cases']} cas comportent respectivement {facts['Counter']['preparation_cycles'].get(1, 0)}, {facts['Counter']['preparation_cycles'].get(2, 0)} et {facts['Counter']['preparation_cycles'].get(3, 0)} observations avec un, deux et trois cycles de préparation. Le modèle conserve cette possibilité de répétition ; Alpha ne borne pas automatiquement la boucle à trois passages.",
            f"Cash précède Give order dans {facts['Counter']['cash_first']} cas ; l’ordre inverse apparaît dans {facts['Counter']['handover_first']} cas. Le routage impose l’accomplissement des deux activités tout en autorisant ces deux ordres.",
        ],
        "Phone": [
            "La commande est enregistrée par téléphone, puis suit la production Prepare–Bake–Pack avec possibilité de répétition. Une branche ajoute Plan puis Deliver avant les activités de règlement et de remise.",
            f"La livraison est enregistrée dans {facts['Phone']['delivered_cases']} des {facts['Phone']['cases']} cas. Les {facts['Phone']['cases'] - facts['Phone']['delivered_cases']} autres ne contiennent pas Plan/Deliver : ils sont interprétés comme un retrait dans le modèle organisationnel, avec cette interprétation signalée comme hypothèse.",
            f"Les cycles de préparation sont répartis ainsi : {facts['Phone']['preparation_cycles'].get(1, 0)} cas à un cycle, {facts['Phone']['preparation_cycles'].get(2, 0)} à deux, {facts['Phone']['preparation_cycles'].get(3, 0)} à trois. Cash est observé avant la remise dans {facts['Phone']['cash_first']} cas, après dans {facts['Phone']['handover_first']} cas.",
        ],
        "Internet": [
            "Receive order by internet est suivi d’Approve order. Une voie poursuit directement ; une autre passe par Contact customer puis soit Modify order, soit Cancel order. Une annulation termine le traitement sans préparation ni livraison.",
            f"Sur {facts['Internet']['cases']} cas, {facts['Internet']['contact_cases']} comportent un contact, {facts['Internet']['modified_cases']} une modification et {facts['Internet']['cancelled_cases']} une annulation. Les {facts['Internet']['cases'] - facts['Internet']['contact_cases']} autres ne comportent pas de contact. Les {facts['Internet']['delivered_cases']} cas non annulés contiennent une livraison.",
            f"Les {facts['Internet']['delivered_cases']} cas préparés présentent {facts['Internet']['preparation_cycles'].get(1, 0)} séquences à un cycle, {facts['Internet']['preparation_cycles'].get(2, 0)} à deux et {facts['Internet']['preparation_cycles'].get(3, 0)} à trois. Le règlement et la remise restent tous deux nécessaires dans le chemin non annulé.",
        ],
    }
    for index, channel in enumerate(CHANNELS, 1):
        blocks.append(heading(f"2.{index} Commande par {CHANNEL_FR[channel].lower()}", 2))
        blocks.extend(paragraph(text) for text in descriptions[channel])
        blocks.append(figure(models[channel]["paths"]["svg"], f"BPMN {CHANNEL_FR[channel]}. Vue d’ensemble ; l’annexe vectorielle et le fichier SVG permettent la lecture détaillée avec zoom."))
        blocks.append(table(["Fichier modifiable", "Échange standard", "Figure vectorielle"], [[
            models[channel]["paths"]["opx"].name, models[channel]["paths"]["bpmn"].name, models[channel]["paths"]["svg"].name
        ]], small=True))
        blocks.append(page())
    blocks.extend([
        heading("3. Intégration dans un processus global"),
        paragraph("Le modèle global possède un seul participant Pizzeria, un démarrage commun, une passerelle exclusive de sélection du canal et une fin commune. Les trois sous-graphes de contrôle sont reliés à ces éléments communs ; les mêmes catégories de rôles et les mêmes interfaces de données organisent l’ensemble."),
        paragraph("Une tâche n’est pas fusionnée avec une autre uniquement parce que son libellé est identique. Cette précaution évite de créer un scénario hybride, par exemple une commande au comptoir qui emprunterait les décisions de contact/annulation propres à Internet. La mutualisation porte ici sur le participant, la sélection d’entrée, la complétion, les rôles et les informations ; les contraintes propres à chaque canal sont conservées."),
        paragraph("Les conditions nommées des branches sont des indications de lecture et de configuration. Elles ne constituent pas un moteur de décision exécutable ni la preuve d’une variable enregistrée dans les traces. La vérification porte sur le contrôle de flux et les activités observables."),
        figure(models["Global"]["paths"]["svg"], "Processus global connecté : trois branches de canal dans un participant partagé. Le modèle détaillé est également fourni en OPX, BPMN 2.0 et SVG."),
        paragraph("L’intégration est contrôlée par rejeu des 327 traces projetées ainsi que par des contre-exemples sélectionnés : sauts de tâches requises, défaut de règlement/remise et passages non autorisés entre canaux. Ces essais complètent les contrôles structurels et les allers-retours de formats ; ils ne constituent pas une preuve exhaustive de précision."), page(),
        heading("4. Dimensions organisationnelle et informationnelle"),
        heading("4.1 Rôles et affectations", 2),
        paragraph("Les noms de personnes proviennent des colonnes Actor. Les intitulés de couloirs marqués d’un astérisque sont des regroupements de modélisation, proposés à partir des activités observées ; ils ne sont pas présentés comme un organigramme officiel."),
        table(["Couloir proposé", "Évidence / responsabilité"], [
            ["Gestion de commande*", "William : prises/réception de commande, approbation, contact, modification et annulation selon le canal"],
            ["Production*", "Patrick et Robert : Prepare, Bake, Pack ; cinq Prepare du comptoir restent sans acteur renseigné"],
            ["Livraison*", "Jack, Paul, Richard et Harry : affectations observées aux activités de planification/livraison"],
            ["Règlement et remise*", "William ou acteur de livraison selon les traces et le canal ; les affectations exactes figurent dans la documentation de chaque tâche"],
        ]),
        paragraph("Un couloir regroupe une responsabilité ; il n’impose ni ressource unique, ni capacité, ni temps de travail. Les attributs de coût et de durée des tâches restent à leur valeur technique par défaut faute de mesures : ces zéros ne sont pas utilisés comme des estimations opérationnelles."),
        paragraph("Le contrôle des cas livrés montre que, pour chacune des 65 livraisons Phone et des 182 livraisons Internet, un même acteur réalise Plan, Deliver, Cash et Give order. Cette continuité observée explique l’affectation du règlement/remise à l’acteur de livraison. Les deux couloirs représentent des responsabilités, pas nécessairement deux employés différents ; les disponibilités et contraintes de ressources restent hors du moteur de contrôle."),
        heading("4.2 Informations et associations", 2),
        paragraph("Les objets et magasins de données expriment une configuration informationnelle documentée. Les attributs enregistrés viennent des feuilles complémentaires ; les associations avec les tâches sont une interprétation explicitement proposée, car le journal ne contient pas de traces de lecture/écriture de données."),
        paragraph("Dans le modèle global, certaines associations sont représentatives afin de garder le schéma lisible. La documentation des objets précise leur portée partagée entre les branches et conserve les affectations propres à chaque canal."),
        table(["Information", "Attributs / origine", "Usage proposé et limite"], [
            ["Commande / canal", "CaseId et activités du CSV ; mode connu par le fichier source", "Piloter le choix du canal ; pas de clé de jointure créée vers Sales"],
            ["Gabarit de scénario", "CaseStructure ; StructureId local dans Sales", "Décrire les scénarios complémentaires ; correspondance Start/Prepare de Phone documentée"],
            ["Vente enregistrée", "Id, StructureId, Pizza Type, Pizza Size, Revenue, Cost", "Conserver le relevé commercial sans assimiler une ligne à un panier ou à une commande du journal"],
            ["Profil déclaré", "Customer Type, Customer Location, Weekday", "Segmenter les observations ; aucune identité unique ni date complète n’est déduite"],
            ["Retour de satisfaction", "Satisfaction, éventuellement absente", "Évaluer les scores observés et leur couverture, en distinguant absence et zéro"],
            ["Catalogue pizza", "PizzasInfo : six types × trois tailles, revenus/coûts de référence", "Documenter la production et les montants ; unité monétaire non indiquée"],
        ], small=True),
        paragraph("Les libellés d’activité et les champs d’origine restent traçables dans evidence.json. Les coquilles et gabarits anciens sont signalés, sans créer de relation commerciale ou informationnelle non démontrée."), page(),
        heading("5. Analyse quantitative"),
        heading("5.1 Clients, transactions et unités de comptage", 2),
        paragraph("Le tableau suivant répond au dénombrement par type en comptant les mentions de type sur les lignes de vente. En l’absence d’identifiant de personne, il ne permet pas de compter des clients distincts. Les 463 lignes constituent le volume commercial observable ; 327 est le nombre séparé de cas événementiels. Le nombre exact de paniers/transactions reliant ces sources n’est pas identifiable."),
        table(["Type enregistré", "Lignes", "Part de l’échantillon", "Satisfaction observée"], [
            [label(row, "customer_type"), row["recorded_sales"], percent(Decimal(row["recorded_sales"]) / Decimal(totals["recorded_sales"])),
             f"{fr_number(row['satisfaction_average'])} (n={row['satisfaction_observed']})"] for row in groups["customers"]
        ]),
        plot("01"), plot("07"),
        paragraph("L’axe lundi–dimanche est un profil par catégorie de jour, calculé sur toutes les observations disponibles. Il ne montre pas une progression datée et ne permet pas de calculer une croissance entre semaines ou une fréquentation quotidienne de clients uniques."),
        heading("5.2 Chiffre d’affaires, coûts et bénéfice", 2),
        table(["Canal", "Ventes", "CA (u.m.)", "Coûts", "Bénéfice", "Bénéfice / CA"],
              metric_rows("channels", "channel") + [["Total", totals["recorded_sales"], fr_number(totals["revenue"]), fr_number(totals["cost"]), fr_number(totals["profit"]), percent(totals["profit_rate"])]], small=True),
        plot("02"),
        paragraph("Le bénéfice calculé correspond à la différence entre les revenus et les seuls coûts enregistrés. Les parts par canal ne constituent pas une mesure de rentabilité nette après frais fixes ou de livraison. Une comparaison de canaux doit donc être complétée par ces coûts si le gérant souhaite une décision économique complète."),
        heading("5.2.1 Annulations et sensibilité financière", 2),
        paragraph("Le rapprochement interne au classeur Internet identifie trois lignes Sales (Id 18, 61 et 135) dont StructureId = 7 référence le scénario Contact customer → Cancel order. Elles portent pourtant des valeurs mises en cache de Revenue et Cost, respectivement 36,00 et 28,00 u.m. au total. Ces champs ne suffisent donc pas à établir des encaissements ou coûts effectivement réalisés. Ce rapprochement de gabarit ne permet pas d’identifier les CaseId correspondants."),
        table(["Périmètre", "Lignes", "Revenue", "Cost", "Différence"], [
            ["Tous les enregistrements (réponse de base)", "463", "6 795,00", "5 420,00", "1 375,00"],
            ["Lignes liées au gabarit d’annulation", "3", "36,00", "28,00", "8,00"],
            ["Sensibilité : hors ces trois lignes", "460", "6 759,00", "5 392,00", "1 367,00"],
        ], small=True),
        paragraph("La dernière ligne est une sensibilité sous l’hypothèse d’exclure les montants du gabarit d’annulation ; elle ne remplace pas les totaux de base et n’est pas une correction comptable validée. Les sources ne documentent ni statut de facturation, ni remboursement, ni règle d’imputation des coûts. Le diagnostic détaillé est joint et les enregistrements d’origine sont conservés."),
        heading("5.3 Profils de vente selon le jour", 2),
        plot("03"),
        table(["Jour", "Ventes", "CA (u.m.)", "Coûts", "Bénéfice", "Bénéfice / CA"],
              [[DAY_FR.get(row[0], row[0]), *row[1:]] for row in metric_rows("weekdays", "weekday")], small=True),
        paragraph("Saturday représente 193 lignes sur 463, soit 41,68 % de l’échantillon ; Saturday et Sunday réunis en représentent 55,72 %. Cette concentration invite à vérifier l’adéquation des moyens de préparation aux jours de forte demande sur une période datée plus longue. Elle ne suffit pas à établir une règle de capacité valable pour toutes les semaines."),
        heading("5.4 Répartition géographique", 2),
        plot("04"),
        table(["Localisation enregistrée", "Ventes", "CA (u.m.)", "Coûts", "Bénéfice", "Bénéfice / CA"], metric_rows("locations", "customer_location"), small=True),
        paragraph("London District One regroupe 256 lignes, soit 55,29 % de l’échantillon. La source fournit des catégories géographiques, pas des coordonnées ni des distances : une répartition par libellé est pertinente, alors qu’une carte de zones ou un coût kilométrique serait inventé."),
        heading("5.5 Satisfaction", 2),
        paragraph(f"La somme des {totals['satisfaction_observed']} scores observés est {fr_number(totals['satisfaction_sum'], 0)} ; la moyenne pondérée est {fr_number(totals['satisfaction_average'], 6)}. Les {totals['satisfaction_missing']} valeurs manquantes sont exclues du dénominateur. Deux valeurs nulles sont de vrais scores zéro et sont conservées. Les trois absences correspondent précisément aux lignes liées au gabarit d’annulation Internet : elles peuvent être non applicables plutôt que des oublis de collecte. L’étendue observée ne suffit pas à documenter le barème du questionnaire."),
        plot("05"),
    ])
    for title, key, dimension in [("Catégorie de clientèle", "customers", "customer_type"), ("Type de pizza", "pizzas", "pizza_type"), ("Région enregistrée", "locations", "customer_location")]:
        blocks.append(table([title, "Moyenne", "Observés", "Manquants"], [[label(row, dimension), fr_number(row["satisfaction_average"], 4), row["satisfaction_observed"], row["satisfaction_missing"]] for row in groups[key]], small=True))
    blocks.extend([
        paragraph("Ces moyennes décrivent des sous-ensembles de tailles inégales. Elles n’établissent pas une causalité entre pizza, lieu, type de clientèle et satisfaction. Les petits groupes, notamment Senior, appellent une lecture prudente et la couverture des scores doit rester visible."),
        heading("5.6 Palmarès des pizzas", 2),
        plot("06"),
        table(["Pizza", "Ventes", "CA (u.m.)", "Coûts", "Bénéfice", "Bénéfice / CA"], metric_rows("pizzas", "pizza_type"), small=True),
        paragraph("Le classement utilise le nombre de lignes de vente ; les ex æquo partagent un rang. Le CA puis le libellé stabilisent leur ordre d’affichage. Salami arrive en tête du volume global avec 130 lignes, devant Margherita (109). Les classements complets par type de client et jour sont joints en CSV."),
        table(["Type de client", *[DAY_FR[day] for day in WEEKDAYS]], [
            [label(customer, "customer_type"), *[
                next((", ".join(group["leaders"]) + " (" + str(group["ranking"][0]["recorded_sales"]) + ")"
                      for group in summary["rankings"]["by_customer_type_weekday"]
                      if group["customer_type"] == label(customer, "customer_type") and group["weekday"] == day and group["ranking"]), "Aucune observation")
                for day in WEEKDAYS]] for customer in groups["customers"]
        ], small=True),
        paragraph("Lecture du tableau : pizza(s) de premier rang pour chaque type et jour, suivie(s) du nombre de lignes de chacune. « Aucune observation » signifie qu’aucune vente du segment n’est enregistrée, pas qu’une pizza a été proposée puis refusée."),
        heading("6. Deux KPI proposés au gérant"),
        heading("6.1 Part du bénéfice dans le CA enregistré", 2),
        paragraph(f"KPI 1 = (CA − coûts enregistrés) / CA. Valeur globale : {percent(totals['profit_rate'])}, calculée avec {fr_number(totals['profit'])} / {fr_number(totals['revenue'])}. Cet indicateur est sans dimension et reste non défini si le CA est nul."),
        paragraph("Il complète le volume : vendre davantage ne garantit pas une meilleure contribution au bénéfice enregistré. Son suivi par canal et pizza aide à repérer un mix moins rémunérateur ou une variation de coût. Un seuil de pilotage autour de 20 % peut servir de point de discussion à partir de cet échantillon ; ce n’est ni un objectif imposé ni une référence du secteur. Avant de décider un prix, il faut intégrer les charges absentes des fichiers."),
        heading("6.2 Taux de satisfaction renseignée", 2),
        paragraph(f"KPI 2 = nombre de lignes avec un score / nombre de lignes de vente. Valeur globale : {percent(Decimal(totals['satisfaction_observed']) / Decimal(totals['recorded_sales']))}, soit {totals['satisfaction_observed']} / {totals['recorded_sales']}. Un score zéro reste renseigné ; un score absent ne l’est pas."),
        paragraph("Ce KPI mesure la couverture de la donnée qui soutient les moyennes de satisfaction. Il évite d’interpréter une moyenne favorable ou défavorable comme représentative lorsque les retours manquent. Les trois absences sont dans le gabarit d’annulation Internet : il faut d’abord vérifier l’applicabilité de la note, puis distinguer « non applicable » de « non répondu », sans fabriquer de score. Sous l’hypothèse d’un périmètre hors annulations, les 460 lignes restantes sont toutes notées, soit 100 % ; cette sensibilité ne change pas le dénominateur du KPI de base."),
        table(["Canal", "Bénéfice / CA", "Couverture satisfaction", "Scores absents"], [[label(row, "channel"), percent(row["profit_rate"]), percent(Decimal(row["satisfaction_observed"]) / Decimal(row["recorded_sales"])), row["satisfaction_missing"]] for row in groups["channels"]]),
        paragraph("Pour une utilisation opérationnelle future, les deux KPI doivent être recalculés à chaque export daté, avec leur périmètre et leurs dénominateurs. La cadence, les seuils et les responsables de suivi sont des propositions à convenir avec le gérant ; les sources actuelles ne les imposent pas."),
        heading("7. Interprétations et pistes d’amélioration"),
        bullets(
            "Capacité de préparation : confronter la forte présence des lignes Saturday au planning réel et à des dates d’exploitation avant de changer les effectifs. Aucune durée de tâche n’est déduite du seul ordre des événements.",
            "Canaux : suivre simultanément volume et part du bénéfice dans le CA ; isoler les frais de livraison, commissions et charges fixes pour une analyse nette qui n’est pas possible aujourd’hui.",
            "Production : instrumenter la raison des retours Prepare–Bake–Pack avant de les qualifier de reprises, de défauts ou de pizzas supplémentaires.",
            "Clientèle : ajouter une clé de transaction et un identifiant de client adapté au besoin de comptage, ainsi que des dates complètes, si le gérant veut mesurer clients uniques, panier moyen et évolution chronologique.",
            "Satisfaction : distinguer la non-applicabilité éventuelle des notes en cas d’annulation et la non-réponse ; documenter le barème, conserver les scores zéro et publier les effectifs qui soutiennent chaque moyenne.",
        ),
        heading("8. Vérifications et reproductibilité"),
        paragraph("Le générateur natif utilise les types, importeurs, conversions, exports et analyses d’Open Process. Il vérifie les modèles et leurs allers-retours OPX/BPMN XML, le rejeu des traces projetées, des contre-exemples sélectionnés et une analyse bornée de bonne formation. Les résultats détaillés se trouvent dans validation.json ; un résultat borné ou indécidable n’est pas transformé en preuve générale."),
        table(["Modèle", "Nœuds", "Flux", "Cas acceptés", "Analyse du réseau"], [[
            row["model"], row["nodes"], row["flows"], row["recorded_case_replays"], row["bounded_soundness"]
        ] for row in [*validation["channels"], validation["global"]]], small=True),
        paragraph("L’analyse bornée renvoie Sound pour les quatre réseaux de contrôle encodés ; les 15 contre-exemples de contrôle et de mélange de canaux retenus sont rejetés. Ce résultat porte sur l’abstraction de flux modélisée, avec ses hypothèses, et non sur les disponibilités humaines ou l’exécution réelle des échanges d’information."),
        paragraph("Les quatre fichiers BPMN XML passent aussi la validation contre les cinq schémas normatifs OMG BPMN 2.0.2, sans diagnostic, avec libxml2 2.15.2. Le relevé joint conserve les URL et empreintes des schémas ainsi que les empreintes des fichiers contrôlés. Cette validation porte sur la grammaire XML et ses types ; elle ne remplace pas les contrôles de comportement et ne garantit pas toutes les fonctions de tout éditeur BPMN." if xsd and len(xsd.get("models", [])) == 4 and all(row.get("valid") for row in xsd["models"]) else "La validation XSD externe n’est pas encore enregistrée ; ce brouillon ne la présente pas comme acquise."),
        paragraph("Les figures et tables Python sont recalculées indépendamment à partir des montants entiers du document Sales. Leurs totaux et agrégations sont confrontés aux CSV natifs. Les empreintes SHA-256 des entrées et du code de publication figurent dans le manifeste joint ; les empreintes FNV du générateur sont identifiées comme de simples empreintes reproductibles, sans prétention cryptographique."),
        table(["Vérification", "Résultat enregistré"], [[key, str(value)] for key, value in verification.get("results_fr", {"État": verification.get("status", "Voir verification.json")}).items()], small=True),
        paragraph("Les instructions de reproduction et la liste exacte des livrables accompagnent le rapport. Les vérifications locales ne valent pas une exécution de toute la matrice CI sur plusieurs systèmes ni une exécution métier réelle."),
        heading("9. Conclusion"),
        paragraph("Les trois processus observés et leur intégration sont livrés sous forme modifiable. Les dimensions de responsabilité et d’information sont configurées en distinguant ce qui est enregistré de ce qui est proposé. L’analyse quantitative répond aux questions avec les unités réellement disponibles, deux KPI explicités et les limites qui conditionnent leur interprétation. L’étape suivante, côté pizzeria, serait d’enrichir l’instrumentation avant de conclure sur les clients uniques, les paniers, les délais ou les capacités."),
        heading("Annexes et fichiers de preuve"),
        bullets(
            "Quatre modèles : takeaway-counter, takeaway-phone, takeaway-internet, takeaway-global, chacun en .opx, .bpmn et .svg.",
            "evidence.json : feuilles complémentaires, scénarios, acteurs, provenance, hypothèses et limites ; validation.json : contrôles des modèles et des traces.",
            "Étude Sales .opx, agrégations et séries/KPI CSV natifs ; tables et figures indépendantes sous analysis/.",
            "Les quatre pages vectorielles grand format qui suivent le corps du PDF sont les diagrammes complets. Elles sont destinées à la lecture avec zoom, sans perte de qualité.",
        ),
    ])
    return blocks


def relative(path: Path, output: Path) -> str:
    import os
    return Path(os.path.relpath(path.resolve(), output.resolve())).as_posix()


def write_html(blocks, destination: Path, output: Path, build_date: str):
    parts = ["<!doctype html><html lang='fr'><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'>",
             "<title>Rapport final — TakeAway</title><style>",
             "body{font:16px/1.6 system-ui,sans-serif;color:#152238;background:#eef2f7;margin:0}main{max-width:1060px;margin:32px auto;padding:48px;background:white}h1,h2,h3{color:#124b64;line-height:1.25}h1{font-size:2.2rem}h2{margin-top:2.2rem;border-bottom:2px solid #d5e8ee;padding-bottom:8px}table{border-collapse:collapse;width:100%;font-size:.88rem;margin:20px 0}th,td{border:1px solid #d7e0e9;text-align:left;vertical-align:top;padding:8px}th{background:#e8f1f6}tr:nth-child(even){background:#f8fafc}figure{margin:28px 0}figure img{width:100%;height:auto;border:1px solid #e2e8f0}figcaption{font-size:.85rem;color:#475569}li{margin:.6rem 0}.date{color:#64748b}.page{margin-top:3rem}@media print{body{background:white}main{max-width:none;margin:0;padding:0}.page{break-before:page}table{font-size:9pt}h2{break-after:avoid}figure{break-inside:avoid}}",
             "</style><main><p class='date'>Édition du " + html.escape(build_date) + "</p>"]
    for block in blocks:
        kind = block["kind"]
        if kind == "heading":
            tag = min(block["level"] + 1, 3)
            parts.append(f"<h{tag}>{html.escape(block['text'])}</h{tag}>")
        elif kind == "p":
            parts.append("<p>" + html.escape(block["text"]) + "</p>")
        elif kind == "bullets":
            parts.append("<ul>" + "".join("<li>" + html.escape(item) + "</li>" for item in block["items"]) + "</ul>")
        elif kind == "table":
            parts.append("<table><thead><tr>" + "".join("<th>" + html.escape(cell) + "</th>" for cell in block["headers"]) + "</tr></thead><tbody>")
            parts.extend("<tr>" + "".join("<td>" + html.escape(cell) + "</td>" for cell in row) + "</tr>" for row in block["rows"])
            parts.append("</tbody></table>")
        elif kind == "figure":
            path = Path(block["path"])
            vector = path.with_suffix(".svg") if path.with_suffix(".svg").exists() else path
            url = html.escape(relative(vector, output), quote=True)
            parts.append(f"<figure><a href='{url}'><img src='{url}' alt='{html.escape(block['caption'], quote=True)}'></a><figcaption>{html.escape(block['caption'])}</figcaption></figure>")
        elif kind == "page":
            parts.append("<div class='page'></div>")
    parts.append("</main></html>\n")
    destination.write_text("\n".join(parts), encoding="utf-8")


def write_pdf(blocks, destination: Path, output: Path, models, build_date: str):
    import cairosvg
    from reportlab import rl_config
    from reportlab.lib import colors
    from reportlab.lib.enums import TA_LEFT
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
    from reportlab.lib.units import mm
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.platypus import Image, KeepTogether, PageBreak, Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle
    from pypdf import PdfReader, PdfWriter

    rl_config.invariant = 1
    for name, filename in [("DejaVu", "DejaVuSans.ttf"), ("DejaVu-Bold", "DejaVuSans-Bold.ttf")]:
        path = Path("/usr/share/fonts/truetype/dejavu") / filename
        if not path.is_file():
            import matplotlib
            path = Path(matplotlib.get_data_path())/"fonts/ttf"/filename
        pdfmetrics.registerFont(TTFont(name, str(path)))
    pdfmetrics.registerFontFamily("DejaVu", normal="DejaVu", bold="DejaVu-Bold", italic="DejaVu", boldItalic="DejaVu-Bold")
    styles = getSampleStyleSheet()
    body = ParagraphStyle("TPBody", fontName="DejaVu", fontSize=9, leading=13, spaceAfter=8, alignment=TA_LEFT)
    caption = ParagraphStyle("TPCaption", parent=body, fontSize=7.5, leading=10, textColor=colors.HexColor("#475569"))
    cell = ParagraphStyle("TPCell", parent=body, fontSize=7.3, leading=10, spaceAfter=1)
    small_cell = ParagraphStyle("TPSmallCell", parent=cell, fontSize=6.5, leading=8.6)
    heads = {level: ParagraphStyle(f"TPH{level}", parent=body, fontName="DejaVu-Bold", fontSize=size, leading=size+5,
                                 spaceBefore=14, spaceAfter=9, textColor=colors.HexColor("#124b64"), keepWithNext=True)
             for level, size in [(0, 24), (1, 16), (2, 12)]}
    width = A4[0] - 36 * mm
    story = [Paragraph("Édition du " + html.escape(build_date), caption), Spacer(1, 8)]
    cache = output / ".rendered"
    cache.mkdir(parents=True, exist_ok=True)
    for block in blocks:
        kind = block["kind"]
        if kind == "heading":
            story.append(Paragraph(html.escape(block["text"]), heads[min(block["level"], 2)]))
        elif kind == "p":
            story.append(Paragraph(html.escape(block["text"]), body))
        elif kind == "bullets":
            for item in block["items"]:
                story.append(Paragraph("• " + html.escape(item), body))
        elif kind == "page":
            story.append(PageBreak())
        elif kind == "table":
            style = small_cell if block["small"] else cell
            rows = [[Paragraph(html.escape(str(value)), style) for value in row] for row in [block["headers"], *block["rows"]]]
            element = Table(rows, colWidths=[width / len(block["headers"])] * len(block["headers"]), repeatRows=1, hAlign="LEFT")
            element.setStyle(TableStyle([
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e8f1f6")),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#f8fafc")]),
                ("GRID", (0, 0), (-1, -1), .35, colors.HexColor("#d7e0e9")),
                ("VALIGN", (0, 0), (-1, -1), "TOP"), ("LEFTPADDING", (0, 0), (-1, -1), 5),
                ("RIGHTPADDING", (0, 0), (-1, -1), 5), ("TOPPADDING", (0, 0), (-1, -1), 5),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
            ]))
            story.extend([element, Spacer(1, 8)])
        elif kind == "figure":
            path = Path(block["path"])
            if path.suffix == ".svg":
                png = cache / (path.stem + ".png")
                cairosvg.svg2png(url=str(path), write_to=str(png), output_width=3600)
                path = png
            image = Image(str(path))
            scale = min(width / image.imageWidth, 170 * mm / image.imageHeight)
            image.drawWidth = image.imageWidth * scale
            image.drawHeight = image.imageHeight * scale
            story.append(KeepTogether([image, Spacer(1, 5), Paragraph(html.escape(block["caption"]), caption), Spacer(1, 10)]))

    def footer(canvas, doc):
        canvas.saveState()
        canvas.setStrokeColor(colors.HexColor("#d7e0e9"))
        canvas.line(18*mm, 15*mm, A4[0]-18*mm, 15*mm)
        canvas.setFont("DejaVu", 7)
        canvas.setFillColor(colors.HexColor("#475569"))
        canvas.drawString(18*mm, 10*mm, "TakeAway · Open Process · 2026/2027")
        canvas.drawRightString(A4[0]-18*mm, 10*mm, f"Page {doc.page}")
        canvas.restoreState()

    body_pdf = cache / "report-body.pdf"
    doc = SimpleDocTemplate(str(body_pdf), pagesize=A4, leftMargin=18*mm, rightMargin=18*mm, topMargin=17*mm,
                            bottomMargin=20*mm, title="TakeAway — rapport final", author="", subject="Process Intelligence")
    doc.build(story, onFirstPage=footer, onLaterPages=footer)
    writer = PdfWriter()
    writer.append(str(body_pdf), outline_item="Rapport : découverte et analyse")
    for channel in (*CHANNELS, "Global"):
        pdf_bytes = cairosvg.svg2pdf(url=str(models[channel]["paths"]["svg"]))
        writer.append(io.BytesIO(pdf_bytes), outline_item=f"BPMN complet — {CHANNEL_FR.get(channel, 'Global')}")
    writer.add_metadata({"/Title": "TakeAway — rapport final", "/Subject": "Process Intelligence : BPMN, organisation, informations et analyse quantitative", "/Author": "", "/Creator": "Open Process report tooling", "/CreationDate": "D:"+build_date.replace("-", "")+"120000+01'00'"})
    with destination.open("wb") as result:
        writer.write(result)
    reader = PdfReader(destination)
    text = "\n".join(page.extract_text() or "" for page in reader.pages)
    for required in ["Synthèse", "Processus découverts", "Intégration", "Satisfaction", "Deux KPI", "Conclusion"]:
        if required not in text:
            raise ValueError(f"Required report content missing from PDF extraction: {required}")
    if len(reader.pages) < 10:
        raise ValueError("Unexpectedly short report")
    shutil.rmtree(cache)
    return {"pages": len(reader.pages), "body_pages": len(reader.pages)-4, "vector_appendices": 4, "required_sections_present": True}


def archive_submission(tp: Path, build_date: str):
    """Keep TP-relative paths so HTML images and the coursework index still work."""
    directory = tp/"submission"
    directory.mkdir(parents=True, exist_ok=True)
    destination = directory/"takeaway-tp.zip"
    entries = [path for path in sorted(tp.rglob("*")) if path.is_file()
               and path.relative_to(tp).parts[0] != "submission"
               and path.suffix not in {".tmp", ".bak", ".pyc"}
               and "__pycache__" not in path.parts]
    stamp = (*date.fromisoformat(build_date).timetuple()[:3], 12, 0, 0)
    inventory = []
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in entries:
            relative_path = path.relative_to(tp).as_posix()
            name = "takeaway-tp/" + relative_path
            info = zipfile.ZipInfo(name, date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            data = path.read_bytes()
            if relative_path == "README.md":
                # An archive cannot include a second copy of itself. Its index
                # links to the contained deliverables rather than the outer ZIP.
                data = ("# TakeAway TP deliverables\n\n"
                        "- [Final PDF report](report/rapport-final.pdf)\n"
                        "- [HTML report and figures](report/rapport-final.html)\n"
                        "- [Delivery checklist](PLAN.md)\n"
                        "- [Reproduction instructions](REPRODUCE.md)\n"
                        "- [Recorded checks](results/verification.json)\n"
                        "- [BPMN evidence](results/bpmn/evidence.json) and [validation](results/bpmn/validation.json)\n\n"
                        "Editable .opx, standard .bpmn and vector .svg models are in results/bpmn/. "
                        "The three corrected CSVs in clean/ are the authoritative behavioral inputs. "
                        "Supplied originals and complementary workbooks are preserved. "
                        "Independent analysis assets are in report/analysis/. "
                        "package-manifest.json lists every contained file and its SHA-256.\n").encode("utf-8")
            archive.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
            inventory.append({"path": relative_path, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)})
        package_manifest = {"schema_version": 1, "files": inventory}
        info = zipfile.ZipInfo("takeaway-tp/package-manifest.json", date_time=stamp)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        archive.writestr(info, json.dumps(package_manifest, ensure_ascii=False, indent=2)+"\n")
    with zipfile.ZipFile(destination) as archive:
        if archive.testzip() is not None:
            raise ValueError("Submission ZIP integrity check failed")
        names = set(archive.namelist())
        import posixpath
        html_source = archive.read("takeaway-tp/report/rapport-final.html").decode()
        for target in re.findall(r"(?:src|href)='([^']+)'", html_source):
            if re.match(r"^[a-z]+:", target):
                continue
            name = posixpath.normpath(posixpath.join("takeaway-tp/report", html.unescape(target)))
            if name not in names:
                raise ValueError(f"HTML asset missing from submission: {target}")
    checksum = sha256(destination)
    (directory/"takeaway-tp.zip.sha256").write_text(checksum+"  takeaway-tp.zip\n", encoding="ascii")
    return {"archive": str(destination), "archive_sha256": checksum, "archive_files": len(inventory)+1}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tp", type=Path, default=ROOT/"tp")
    parser.add_argument("--output", type=Path, default=ROOT/"tp/report")
    parser.add_argument("--analysis", type=Path, default=ROOT/"tp/report/analysis")
    parser.add_argument("--draft", action="store_true", help="Clearly marked preview while application checks are running")
    parser.add_argument("--date", default="2026-11-20", help="Publication date, independent of the undated sales data")
    args = parser.parse_args()
    date.fromisoformat(args.date)
    tp, output = args.tp.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    analysis_root = args.analysis.resolve()
    summary = load_json(analysis_root/"summary.json")
    validation = load_json(tp/"results/bpmn/validation.json")
    if not validation.get("all_327_cases_accepted_by_channel_and_global_bpmn"):
        raise ValueError("Native model/trace validation is required before publishing")
    evidence = load_json(tp/"results/bpmn/evidence.json")
    for source in evidence["inputs"]:
        for path_key, fingerprint_key in [("authoritative_event_log", "event_log_fingerprint"), ("complementary_workbook", "workbook_fingerprint")]:
            path = ROOT/source[path_key]
            if native_fingerprint(path) != source[fingerprint_key]:
                raise ValueError(f"Source changed; regenerate native evidence before publishing: {path}")
    if sha256(tp/"results/sales/takeaway-sales.opx") != summary["study"]["sha256"]:
        raise ValueError("Sales study changed; regenerate independent analysis before publishing")
    verification_path = tp/"results/verification.json"
    verification = load_json(verification_path) if verification_path.exists() else {"all_required_checks_passed": False, "status": "Vérifications applicatives en cours"}
    if not args.draft and not verification.get("all_required_checks_passed"):
        raise ValueError("Application verification must pass before publishing")
    if not args.draft:
        for name, expected in verification.get("source_hashes", {}).items():
            if sha256(ROOT/name) != expected:
                raise ValueError(f"Software changed after verification: {name}")
    if not args.draft and (not summary["validation"]["success"] or summary["validation"]["checked_native_csv_count"] != 8):
        raise ValueError("All eight native CSV comparisons must pass before publishing")
    if not args.draft and (not summary.get("scenario_diagnostics", {}).get("success") or not summary["scenario_diagnostics"]["native_subset_validation"]["success"]):
        raise ValueError("The cancellation-template diagnostic must match the native scenario export")
    if not args.draft and summary["scenario_diagnostics"]["evidence_sha256"] != sha256(tp/"results/bpmn/evidence.json"):
        raise ValueError("Native evidence changed; regenerate scenario diagnostics before publishing")
    xsd_path = tp/"results/checks/bpmn-xsd.json"
    xsd = load_json(xsd_path) if xsd_path.exists() else {}
    if not args.draft:
        if not xsd.get("schema_compilation_succeeded") or len(xsd.get("models", [])) != 4 or not all(row.get("valid") for row in xsd["models"]):
            raise ValueError("All four BPMN XML files must pass the recorded official-schema validation")
        for row in xsd["models"]:
            if sha256(ROOT/row["path"]) != row["sha256"]:
                raise ValueError(f"BPMN XML changed after schema validation: {row['path']}")
    models = model_files(tp/"results/bpmn")
    facts = event_facts(tp)
    content = sections(summary, facts, models, validation, verification, analysis_root, xsd)
    if args.draft:
        content.insert(0, paragraph("BROUILLON — vérifications applicatives en cours ; ce document n’est pas la restitution finale."))
    html_path = output/"rapport-final.html"
    pdf_path = output/"rapport-final.pdf"
    write_html(content, html_path, output, args.date)
    pdf_validation = write_pdf(content, pdf_path, output, models, args.date)
    manifest = {"schema_version": 1, "publication_date": args.date, "event_facts": facts,
                "pdf_validation": pdf_validation, "sources": {}, "publication_tools": {}}
    sources = [*(tp/"clean").glob("*.csv"), *(tp/"clean").glob("*.xlsx"),
               tp/"results/sales/takeaway-sales.opx", analysis_root/"summary.json",
               tp/"results/bpmn/evidence.json", tp/"results/bpmn/validation.json",
               tp/"results/checks/bpmn-xsd.json",
               ROOT/"scripts/takeaway_report.py", ROOT/"scripts/takeaway_analysis.py",
               ROOT/"scripts/takeaway-report-requirements.txt", ROOT/"crates/op-core/src/convert.rs",
               ROOT/"crates/op-core/src/sales.rs", ROOT/"crates/op-core/src/bpmn/svg.rs",
               ROOT/"crates/op-core/src/bpmn.rs", ROOT/"crates/op-core/src/bpmn_xml.rs",
               ROOT/"crates/op-core/examples/takeaway_tp.rs", ROOT/"crates/op-core/examples/takeaway_tp/support.rs"]
    sources.extend(path for model in models.values() for path in model["paths"].values())
    if verification_path.exists():
        sources.append(verification_path)
    for path in sorted(sources):
        manifest["sources"][str(path.relative_to(ROOT))] = sha256(path)
    import importlib.metadata
    for name in ["matplotlib", "reportlab", "cairosvg", "pypdf"]:
        manifest["publication_tools"][name] = importlib.metadata.version(name)
    (output/"manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    package = archive_submission(tp, args.date) if not args.draft else {}
    print(json.dumps({"report_pdf": str(pdf_path), "report_html": str(html_path), **pdf_validation, **package}, ensure_ascii=False))


if __name__ == "__main__":
    main()
