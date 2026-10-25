-- Templates are gone: the tables 0011 and 0015 made, and the stored templates
-- in them, are dropped. The assets table first, which references the other.
DROP TABLE template_assets;
DROP TABLE templates;
