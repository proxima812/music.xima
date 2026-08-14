-- Пресеты умных плейлистов на русском (интерфейс приложения — русский).
--
-- Переименовываем только те строки, которые всё ещё носят исходное английское
-- имя: если пользователь уже переименовал пресет, его выбор важнее нашего.

UPDATE smart_playlists SET name = 'Недавно добавленные' WHERE name = 'Recently Added';
UPDATE smart_playlists SET name = 'Ни разу не играли'   WHERE name = 'Not Played';
UPDATE smart_playlists SET name = 'Забытые'             WHERE name = 'Forgotten';
UPDATE smart_playlists SET name = 'Избранное'           WHERE name = 'Favorites';
UPDATE smart_playlists SET name = 'Часто слушаю'        WHERE name = 'Most Played';
UPDATE smart_playlists SET name = '2020-е'              WHERE name = '2020s';
UPDATE smart_playlists SET name = 'Высокое качество'    WHERE name = 'High Quality';
