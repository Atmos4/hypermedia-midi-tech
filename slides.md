---
theme: default
title: Hypermédia
lang: fr
class: text-center
transition: slide-left
mdc: true
---


# Hypermédia
Midi tech - 15/10/2026

<style>
a{border:none}
</style>


_Arnaud Perrin_  
arnaud.perrin.co@gmail.com



<!--
Total : ~4 min d'intro, ~12 min de démo, ~8 min de concepts, ~5 min de compromis + questions.
-->

---

# Définition
Hypermédia : médium d'information non-linéaire

<v-click>

**Origines**

</v-click>

<v-clicks>

- Années 1960 : Ted Nelson invente le terme
- 2000 : thèse de Roy Fielding, naissance de **REST**

</v-clicks>

<v-click>

**Concepts**

</v-click>

<v-clicks>

- Ressources + Contrôles
- Interface uniforme
- Sans état = HATEOS

</v-clicks>

<!--
Phrase optionnelle : en 2008, Fielding s'est plaint que la plupart des « API REST » ignorent cette contrainte.
-->

---

# Le modèle de maturité de Richardson

<v-clicks>

- **Niveau 0** : pas d'organisation
- **Niveau 1** : concept de ressource
- **Niveau 2** : verbes HTTP
- **Niveau 3** : contrôles hypermédia

</v-clicks>

<v-click>

<div class="pt-6 text-xl">La plupart des API s'arrêtent au niveau 2.</div>


</v-click>

<br>
<br>
<br>
<br>

<div text-center>

<v-click>
  
  ### Aujourd'hui : niveau 3.

</v-click>
  
</div>

<div class="absolute bottom-6 text-sm opacity-50">Modèle de Leonard Richardson, popularisé par Martin Fowler</div>

---
layout: center
class: text-center
---

# Le dispositif

Un serveur. Deux clients lourds : **A** (traditionnel), **B** (hypermédia)

<!--
Deux fenêtres desktop natives et un onglet de navigateur. Le serveur est en TypeScript, les clients en Rust : ils ne partagent que le contrat.
L'un des clients desktop a été écrit à l'ancienne.
-->

---
layout: center
class: text-center
---

# Que s'est-il passé ?

---

# Client A : codé en dur

```json {all|3}
{
  "id": 1,
  "status": "paid",
  "total": 100,
}
```

<v-clicks>

- Le client A ne lit que `status`
- Ses propres règles choisissent les boutons : **draft** → Pay, Apply coupon, Cancel · **paid** → Cancel, Ship

</v-clicks>

<br>
<br>
<br>

<div text-center>

<v-clicks>

**La connaissance des actions est côté client**

</v-clicks>

</div>

<!--
Le 409 du serveur nous protège, mais l'utilisateur voit un bouton qui échoue.
-->

---

# Client B : générique

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

- Le client génère **un bouton par entrée**
- Cliquer sur un bouton envoie une requête vers `href`, avec `method`
- Aucune notion métier

</v-clicks>

<div text-center>

<v-clicks>

**C'est le concept d'interface uniforme**

</v-clicks>

</div>

---

# Client B : une autre ressource

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

<div text-center>

<v-clicks>

**Concept de découplage**

</v-clicks>

</div>

---

# Le Web

<v-clicks>

- HTML est un hypermédia
- Les navigateurs Web sont **des clients hypermédia**
- Découplage du contenu des sites

</v-clicks>

<br>

<v-clicks>

Mais alors...

<div text-center>

## Pourquoi le concept d'hypermédia a-t-il été oublié ?

</div>

</v-clicks>

---

# HTML
Un hypermédia limité

<v-clicks>

- Seuls `<a>` et `<form>` peuvent faire des requêtes
- Seuls le clic et la soumission les déclenchent
- Seulement GET et POST
- Chaque réponse remplace toute la page

</v-clicks>

---

# JSON
Le format Web populaire

<v-clicks>

- Moins verbeux que l'HTML
- Popularisation dans les APIs serveur
- Uniformisation de Javascript
- Clients interactifs = début des SPAs

</v-clicks>

---

# Le renouveau de l'hypermédia


<v-clicks>

Le concept est resté populaire dans les frameworks Web "classiques"
- Hotwire (Ruby on Rails)
- LiveWire (Lavarel, PHP)
- LiveView (Phoenix, Elixir)

Émergence de nouvelles librairies standalone
- htmx
- Datastar

(Exemple en annexe)

</v-clicks>


---

# Quand utiliser l'hypermédia?

### Éviter si

<v-clicks>

- Interfaces destinées à des machines (API externes)
- Mises à jour fréquentes
- État interne indispensable (fonctionnalités hors-ligne)

</v-clicks>

### Bons cas d'utilisations

<v-clicks>

- Interfaces destinées à des humains
- Ressources claires
- Composition de systèmes découplés

</v-clicks>

---
layout: center
class: text-center
---

# Conclusion

---
layout: end
---

# Merci !
Des questions ?

<!--
Préparation :
- GraphQL/OpenAPI ? Ils décrivent un contrat figé. L'hypermédia décrit les prochaines étapes valides à l'exécution.
- Faut-il un format standard ? Non, mais un standard donne des clients réutilisables.
- Versioning ? Il est réduit pour les changements de workflow, pas pour les changements de forme des données.
-->

---

# Annexe: Exemple htmx

```html
<button hx-post="/orders/1/pay" hx-target="#order" hx-swap="outerHTML">
  Pay
</button>
```

<v-click>

« Au clic, fais un POST ici, et remplace `#order` par le HTML qui revient. »

</v-click>
