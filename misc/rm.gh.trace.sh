git rm --cached .env # removes from git index but keeps local copy
echo ".env" >>.gitignore
git add .gitignore
git commit -m "Remove .env and add to .gitignore"
git filter-repo --path config/.env.market_maker.ex --path config/.env.moni.ex --invert-paths
