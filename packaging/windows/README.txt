planrs pour Windows (x86_64)

- planrs-desktop.exe : l'application (double-clic).
- planrs.exe : la ligne de commande et le serveur MCP (planrs mcp), à lancer
  depuis un terminal.

Les binaires ne sont pas signés : au premier lancement, Microsoft Defender
SmartScreen affiche « Windows a protégé votre ordinateur ». Cliquer sur
« Informations complémentaires », puis sur « Exécuter quand même ».

planrs-desktop.exe --version n'affiche rien dans un terminal (application
fenêtrée) ; utiliser planrs.exe --version, ou planrs-desktop.exe --version | more.

Guide complet (empreintes SHA256SUMS comprises) : docs/user/install.md du
dépôt, https://github.com/theodu78/planrs
