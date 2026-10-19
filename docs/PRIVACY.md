<!-- privacy-version: 3 -->
<!-- Maintainer: written to match what the code does; have it checked by someone who knows the GDPR. -->
# Privacy Statement

**Omsi-Hub. Version 3, effective 19 October 2026.**

## 1. Introduction

This Privacy Statement describes how Omsi-Hub processes personal data in connection with:

- the Omsi-Hub application and game software (the **Software**);
- the optional Omsi-Hub online account and its associated services (the **Services**);
- the website at omsihub.com (the **Website**);
- the Omsi-Hub server on Discord and its bot (the **Discord server**).

It is issued in accordance with Regulation (EU) 2016/679, the General Data Protection Regulation (**GDPR**).

The Software and the Website present the same text. The English version of this Privacy Statement is the binding version.

## 2. Controller

The controller responsible for the processing of personal data described in this Privacy Statement is Omsi-Hub, a non-commercial add-on for the openOMSI bus simulator.

Enquiries concerning this Privacy Statement or the processing of personal data may be sent to **info@omsihub.com**.

## 3. Use of the Software without an account

The Software can be used in full without creating an account. In that case all game data, including companies, careers and settings, is stored exclusively on the user's own device, and Omsi-Hub does not collect any personal data. The following limited network communication nevertheless occurs.

### 3.1 Update checks

The Software periodically retrieves information about available versions from GitHub, Inc., and downloads updates from it. GitHub processes the user's IP address in this context, as it would for any request to its servers.

### 3.2 Usage counter

While the game is running, it reports to the openOMSI project's usage counter that an instance is in use. This report contains only:

- a random identifier, generated anew at every start and not stored;
- the version number of the Software;
- the type of operating system.

No name, device identifier or other personal data is transmitted. Users may disable this function at any time under Settings → General.

### 3.3 Multiplayer

When a user shares a company session or joins one, the participating installations connect to each other directly. Each participant to whom the user provides a session code can therefore see the user's IP address. These connections do not pass through servers operated by Omsi-Hub.

### 3.4 Phone and tablet displays

The optional phone and tablet displays operate exclusively within the user's local network.

## 4. Personal data processed for an account

Creating an account is optional. It is required only for the friends list, invitations, the leaderboards, cloud saves and linking a Discord account. For account holders, Omsi-Hub processes the following categories of personal data.

| Category of data | Purpose of processing | Legal basis |
| --- | --- | --- |
| Email address and password (the password is stored only in hashed form) | Authentication, verification of the email address, password recovery | Performance of a contract (Art. 6(1)(b) GDPR) |
| Identifier and email address received from Google, if the user chooses to sign in with a Google account | Authentication | Performance of a contract (Art. 6(1)(b) GDPR) |
| Identifier, username, email address and the address of the profile picture received from Discord, if the user chooses to sign in with a Discord account | Authentication | Performance of a contract (Art. 6(1)(b) GDPR) |
| The identifier and username of a linked Discord account, and the date and time of linking | Giving the user their roles on the Discord server and linking their tickets there to the account (section 4.2); only once the user has linked a Discord account, and the user may unlink it at any time | Consent (Art. 6(1)(a) GDPR) |
| Display name and friend code | Enabling other users to find and identify the account holder | Performance of a contract (Art. 6(1)(b) GDPR) |
| Friends, friend requests and blocked users | Providing the friends list | Performance of a contract (Art. 6(1)(b) GDPR) |
| Online status, time of last activity, and the session code while hosting a session | Informing friends of availability and allowing them to join; visible to friends only | Performance of a contract (Art. 6(1)(b) GDPR) |
| Invitations to a session (they can no longer be accepted after 15 minutes) | Allowing a friend to be invited into a session | Performance of a contract (Art. 6(1)(b) GDPR) |
| Career statistics: points, level, number of duties, minutes driven, kilometres | Displaying the user's profile | Performance of a contract (Art. 6(1)(b) GDPR) |
| The same career statistics, on the public leaderboard | Publication on the leaderboard; only once the user has enabled this, and the user may disable it at any time | Consent (Art. 6(1)(a) GDPR) |
| A game company's name, short name, colours, country, map, game mode and figures (cash, monthly profit, buses, lines, staff, passengers, punctuality, reputation), together with the display name, on the public company leaderboard | Publication on the company leaderboard; only for a company for which the user has enabled this, and the user may disable it at any time, which removes the company from the leaderboard | Consent (Art. 6(1)(a) GDPR) |
| Up to two cloud saves of game companies | Enabling the user to continue playing on another device; accessible to the account holder only | Performance of a contract (Art. 6(1)(b) GDPR) |
| The version of this Privacy Statement accepted, and the date and time of acceptance | Demonstrating that the user has been informed and has agreed | Legal obligation and legitimate interest (Art. 6(1)(c) and (f) GDPR) |

### 4.1 Bug reports

Anyone may send a bug report, from the Website or the Software, with or without an account. A bug report contains the description entered by the user and the category chosen; the page, browser (user agent) and language, or the version of the Software and the operating system; the account identifier when the user is signed in, or otherwise an email address only if the user provides one; and a salted hash of the sender's IP address, used solely to limit the number of reports. Bug reports are processed to correct errors and to reply to the reporter, on the basis of legitimate interest (Art. 6(1)(f) GDPR) or, for account holders, the performance of a contract (Art. 6(1)(b) GDPR).

### 4.2 Discord

Omsi-Hub runs a server on Discord, a service of Discord Inc. Joining it is optional and subject to Discord's own terms and privacy policy; Discord processes everything posted on the server as an independent controller. The Omsi-Hub bot on the Discord server processes personal data as follows.

- **Linking a Discord account.** A user may link a Discord account to their account on the Website; signing in with Discord links that Discord account automatically. Omsi-Hub then stores the Discord account's identifier and username and the date and time of linking. As long as the accounts are linked, the bot gives the user roles on the Discord server that every member of the server can see: "Verified Driver", the rank that follows from the level in the user's career statistics, and the team role for members of the Omsi-Hub team. When linking, Discord also grants a one-time permission to add the user to the Discord server if they are not yet a member; this permission is used once and not stored, and no other Discord token is stored either. The user may unlink at any time on the Website (My Omsi-Hub → Profile → Discord); the link is then deleted and the roles are removed.
- **Commands.** The /profile command shows a linked user's career statistics on the Discord server only if the user has enabled publication on the public leaderboard. The /leaderboard command shows the public leaderboard.
- **Tickets.** A ticket opened on the Discord server is a bug report (section 4.1). It contains the description and category entered, optionally the version of the Software and the operating system, the language setting of the user's Discord app, the identifier and username of the Discord account, and the messages, screenshots and videos posted in the ticket's private thread. Only the user who opened the ticket and the Omsi-Hub team can see it. If a Discord account is linked, the ticket belongs to that account and is also shown on the Website; answers given on the Website are posted in the thread. Tickets are processed on the same legal basis as other bug reports.
- **Moderation.** Discord's automatic moderation, as configured by Omsi-Hub, blocks spam, scam links, invitations to other servers and offensive words, may briefly prevent a member from posting, and reports what it blocked to the moderators. When a member uses "Report to moderators" on a message, the message, its author and the reporting member are passed to the moderators' channel on the Discord server. This processing is based on Omsi-Hub's legitimate interest in a safe community (Art. 6(1)(f) GDPR).

Omsi-Hub does not use personal data for automated decision-making or profiling, does not sell personal data and does not display advertising.

## 5. Recipients and processors

Omsi-Hub engages the following parties in the processing of personal data.

| Party | Role and data concerned | Location |
| --- | --- | --- |
| Supabase, Inc. | Processor: hosting of accounts, the database, cloud saves and bug reports, and the Website's release information | Frankfurt, Germany (European Union) |
| Omsi-Hub's email service provider | Processor: sending verification and password-recovery emails, and delivering bug reports to info@omsihub.com (with the reporter's address as reply address, where known) | Under a data processing agreement |
| The hosting provider of omsihub.com | Processor: serving the Website; server logs may briefly contain IP addresses | Under a data processing agreement |
| Google LLC | Independent controller, only where the user chooses to sign in with a Google account; Google's own privacy policy applies | United States |
| Discord Inc. | Independent controller for the Discord server and, where the user chooses, for signing in with a Discord account; Discord's own privacy policy applies. Answers to tickets from the Discord server are passed to Discord to be posted in their thread | United States |
| GitHub, Inc. | Independent controller for the Software's update checks and for downloads the user starts | United States |

Where personal data is transferred to a country outside the European Economic Area, this takes place on the basis of an adequacy decision of the European Commission, including the EU–U.S. Data Privacy Framework, or the Standard Contractual Clauses adopted by the European Commission.

## 6. Retention periods

- **Account data** is retained for as long as the account exists.
- **Invitations** can no longer be accepted after 15 minutes. They are deleted when dismissed, and together with the account.
- **Bug reports** are retained for 12 months and then deleted; a report linked to an account is erased together with that account.
- **A linked Discord account** is retained until the user unlinks it or deletes the account; its roles on the Discord server are then removed.
- **Tickets from the Discord server** are retained and erased as bug reports are; their private thread on Discord is deleted together with the ticket.
- **Moderation reports** in the moderators' channel on the Discord server are deleted once they have been dealt with, and at the latest after six months.
- **Company leaderboard entries** are removed as soon as the user disables publication, when the company is deleted, or together with the account; entries not updated for 30 days are no longer shown.
- **Deleting the account.** The user may delete the account at any time, in the Software (Account → Delete account) or on the Website. All associated personal data is then erased without delay: the profile, friends, invitations, statistics, leaderboard entries, bug reports and their Discord threads, the Discord link and cloud saves.
- **Proof of consent** to this Privacy Statement is erased together with the account.

## 7. Security

Omsi-Hub takes appropriate technical and organisational measures to protect personal data against loss and unlawful processing:

- all communication is encrypted using HTTPS (TLS);
- access to data is restricted at database level by row-level security, so that each account can access only its own data and the data its friends are permitted to see;
- passwords are stored only in hashed form;
- the Software and the Website contain no confidential keys.

## 8. Rights of data subjects

Under the GDPR, the user has the following rights in respect of their personal data:

- the right of access (Art. 15);
- the right to rectification (Art. 16);
- the right to erasure (Art. 17);
- the right to restriction of processing (Art. 18);
- the right to data portability (Art. 20);
- the right to object (Art. 21).

Where processing is based on consent, the user may withdraw that consent at any time, without affecting the lawfulness of processing carried out before its withdrawal.

Many of these rights can be exercised directly in the Software or on the Website. Other requests may be sent to info@omsihub.com. Omsi-Hub will respond within one month of receipt.

The user also has the right to lodge a complaint with a supervisory authority. In the Netherlands, this is the Autoriteit Persoonsgegevens (autoriteitpersoonsgegevens.nl).

## 9. Minors

An account may be created only by persons aged 16 or over, or with the consent of a parent or legal guardian; the user confirms this when creating an account. The Software may be used without an account at any age.

## 10. Cookies and local storage

The Website does not use cookies. In the user's own browser it stores only what is strictly necessary for the Website and the Services: in local storage, the session token that keeps the user signed in (removed when the user signs out) and the chosen theme and language; in session storage, which is cleared when the browser tab is closed, a cache of release information and short-lived sign-in state (the email address awaiting a verification code and the acceptance of this Privacy Statement during sign-up).

## 11. Amendments

Omsi-Hub may amend this Privacy Statement, in particular when the processing of personal data changes. Each amended version carries a new version number. Before an account can be used again after a material amendment, the Software and the Website ask the user to accept the new version.
