---
theme: default
title: Hypermédia - laissez le serveur piloter
lang: fr
class: text-center
transition: slide-left
mdc: true
---

# Hypermédia

Laissez le serveur piloter

<div class="pt-8 opacity-60">30 minutes · une démo en direct</div>

<!--
Total : ~4 min d'intro, ~12 min de démo, ~8 min de concepts, ~5 min de compromis + questions.
-->

---

# Le problème

```js {all|2|3-4}
function render(order) {
  if (order.status === "paid") {
    showCancelButton()
    showShipButton()
  }
}
```

<v-click>

Chaque changement de règle métier oblige à modifier le serveur **et** tous les clients, en même temps.

</v-click>

<!--
Demander à la salle : qui a déjà livré un changement backend qui a cassé une appli mobile ?
Phrase clé : on parle d'API REST, mais ça se comporte comme une bibliothèque partagée avec un réseau au milieu.
-->

---

# Le web a déjà résolu ce problème

<v-clicks>

- Années 1960 : Ted Nelson invente le terme **hypermédia**
- 2000 : la thèse de Roy Fielding formalise l'architecture du web sous le nom de **REST**
- Le serveur indique au client ce qu'il peut faire ensuite, dans la réponse elle-même
- Les navigateurs n'ont aucune logique propre à chaque site. **Vos clients d'API, si.**

</v-clicks>

<!--
Phrase optionnelle : en 2008, Fielding s'est plaint que la plupart des « API REST » ignorent cette contrainte.
Si le temps presse, réduire à une phrase et fusionner avec la slide suivante.
-->

---

# Le modèle de maturité de Richardson

<v-clicks>

- **Niveau 0** : un seul endpoint, du RPC sur HTTP
- **Niveau 1** : des ressources, chacune avec sa propre URL
- **Niveau 2** : verbes HTTP et codes de statut
- **Niveau 3** : contrôles hypermédia

</v-clicks>

<v-click>

<div class="pt-6 text-xl">La plupart des API s'arrêtent au niveau 2. <b>Aujourd'hui : le niveau 3.</b></div>

</v-click>

<div class="absolute bottom-6 text-sm opacity-50">Modèle de Leonard Richardson, popularisé par Martin Fowler</div>

---
layout: center
class: text-center
---

# Le dispositif

Un serveur. Trois clients : **A**, **B**, **C**.

<div class="pt-6 opacity-80">A et B sont des applications desktop natives en Rust, comme n'importe quel client lourd.<br>C tourne dans un onglet de navigateur.<br>Je ne vous montre que le serveur.</div>

<!--
Deux fenêtres desktop natives et un onglet de navigateur. Le serveur est en TypeScript, les clients en Rust : ils ne partagent que le contrat. Appeler C « celui en HTML » pour l'instant.
L'un des clients desktop a été écrit à l'ancienne.
Demander : lequel survivra à un changement de règle, selon vous ?
-->

---

# Le serveur : une seule fonction décide

```ts {all|3-4|6-7|9-10|12-13}
function action_list(o: Order): ActionItem[] {
  return [{
    name: "pay", title: "Pay",
    enabled: o.status === "draft"
  }, {
    name: "coupon", title: "Apply 10% coupon",
    enabled: o.status === "draft" && !o.coupon
  }, {
    name: "cancel", title: "Cancel",
    enabled: o.status === "draft" || o.status === "paid"
  }, {
    name: "ship", title: "Ship",
    enabled: o.status === "paid"
  }];
}
```

<v-click>

Chaque action dit **quand** elle est disponible. Le serveur n'envoie que celles qui sont activées.

</v-click>

<!--
Montrer la réponse JSON brute (curl ou navigateur) pour qu'ils voient le tableau actions.
-->

---

# Démo en direct

<v-clicks>

1. Faire passer une commande de draft à paid puis shipped dans les trois clients
2. Moment pronostic : qui survit au prochain changement ?
3. Modifier `action_list()` en direct : une commande payée **ne peut plus être annulée**
4. Recharger. Quels clients affichent encore Cancel ? Que se passe-t-il quand on le choisit ?
5. Bonus : ajouter une action `refund`, puis pointer le client B vers `/tickets/1`

</v-clicks>

<!--
Plan B : garder un court enregistrement d'écran au cas où la modification en direct échoue.
-->

---
layout: center
class: text-center
---

# Que s'est-il passé ?

Ouvrons les boîtes noires.

---

# Client A : codé en dur

Ce que le serveur renvoie pour une commande payée, après notre changement de règle :

```json {all|3|5-7}
{
  "id": 1,
  "status": "paid",
  "total": 100,
  "actions": [
    { "name": "ship", "title": "Ship", "method": "POST", "href": "/orders/1/ship" }
  ]
}
```

<v-clicks>

- Le client A ne lit que `status`
- Il ignore complètement `actions`
- Ses propres règles choisissent les boutons : **draft** → Pay, Apply coupon, Cancel · **paid** → Cancel, Ship
- Le serveur ne propose plus Cancel. **Le client A l'affiche toujours.**

</v-clicks>

<!--
Le 409 du serveur nous protège, mais l'utilisateur voit un bouton qui échoue.
-->

---

# Client B : générique

La même réponse, lue par un autre client :

```json {all|5-7|6}
{
  "id": 1,
  "status": "paid",
  "total": 100,
  "actions": [
    { "name": "ship", "title": "Ship", "method": "POST", "href": "/orders/1/ship" }
  ]
}
```

<v-clicks>

- Il lit `actions` : **un bouton par entrée**
- Cliquer sur un bouton envoie une requête vers son `href`, avec sa `method`
- Aucune notion de « paid », « draft », ni même de « commande »
- C'est l'**interface uniforme** en action

</v-clicks>

---

# Client B : une autre ressource

On pointe le même client vers `/tickets/1` :

```json {all|6-8}
{
  "id": 1,
  "title": "Printer is on fire",
  "state": "open",
  "assignee": null,
  "actions": [
    { "name": "assign", "title": "Assign to Alice", "method": "POST", "href": "/tickets/1/assign" }
  ]
}
```

<v-clicks>

- Le client B n'a jamais entendu parler des tickets
- Il affiche quand même le bon bouton et suit le bon lien
- Rien n'a été modifié, rien n'a été redéployé

</v-clicks>

<!--
Moment bonus : lancer le client B sur /tickets/1 en direct.
-->

---

# Client C : HTML + htmx

<v-clicks>

- HTML est un format hypermédia, mais avec des limites
- Seuls `<a>` et `<form>` peuvent faire des requêtes
- Seuls le clic et la soumission les déclenchent
- Seulement GET et POST, et chaque réponse remplace toute la page

</v-clicks>

<v-click>

**htmx lève ces limites.** Ce sont les contrôles hypermédia du HTML, enfin complets.
~14 Ko, une balise script, pas de build, pas d'état côté client.

</v-click>

---

# Une seule ligne suffit à comprendre le modèle

```html
<button hx-post="/orders/1/pay" hx-target="#order" hx-swap="outerHTML">
  Pay
</button>
```

<v-click>

« Au clic, fais un POST ici, et remplace `#order` par le HTML qui revient. »

</v-click>

<v-click>

<div class="pt-4">Même backend, autre format : l'hypermédia est une propriété de la <b>conception</b>, pas du HTML.</div>

</v-click>

<!--
Rester sous ~90 secondes sur la syntaxe htmx. Le sujet, c'est l'hypermédia, pas htmx.
Anticiper « c'est pas encore du jQuery/PHP ? » : le serveur envoie des contrôles adaptés à l'état, pas seulement du contenu.
À citer si besoin : Hotwire/Turbo, Unpoly.
-->

---

# Les concepts, reliés à ce que vous avez vu

| Concept | Dans la démo |
|---|---|
| Contrôles hypermédia | le tableau `actions` / les boutons htmx |
| HATEOAS | le serveur pilote l'état ; le client ne connaissait qu'une URL |
| Interface uniforme | un client générique, commandes et tickets |
| Indépendance du format | HTML et JSON depuis un même backend ; pas besoin de navigateur |

---

# Ce que le client doit encore savoir

<v-clicks>

- L'hypermédia **déplace** le couplage, il ne le supprime pas
- Le client ne sait plus *quand* une action est valide
- Il doit toujours savoir ce qu'une action **signifie**
- Meilleurs cas : interfaces destinées à des humains, workflows qui évoluent, clients qu'on ne peut pas mettre à jour

</v-clicks>

---

# Les coûts

<v-clicks>

- Des réponses plus volumineuses : des contrôles dans chaque réponse
- Un outillage plus mince qu'OpenAPI : moins de générateurs et de bibliothèques clientes
- Peu de gens connaissent : effort de recrutement et d'onboarding
- Peu d'intérêt pour des clients machine au comportement figé
- Choisir un format (HAL, Siren, JSON:API, maison) est un vrai travail

</v-clicks>

---
layout: center
---

# À retenir

<v-clicks>

1. Demandez-vous **qui décide de ce qui est possible ensuite ?** Laissez le serveur répondre.
2. Pas besoin de framework. Ajoutez une liste `actions` à une ressource.
3. htmx est déjà un client hypermédia.

</v-clicks>

---
layout: center
class: text-center
---

# Des questions ?

<!--
Préparation :
- GraphQL/OpenAPI ? Ils décrivent un contrat figé. L'hypermédia décrit les prochaines étapes valides à l'exécution.
- Faut-il un format standard ? Non, mais un standard donne des clients réutilisables.
- Versioning ? Il est réduit pour les changements de workflow, pas pour les changements de forme des données.
- Pourquoi ça n'a pas pris ? L'outillage, l'ère des SPA qui a favorisé les contrats JSON, et « REST » redéfini comme le niveau 2.
-->
