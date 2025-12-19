git add .
git commit -a -m"Tests" --author="Denis Stepulenok <Denis.Stepulenok@kuehne-nagel.com>"
git pull
git push origin 
codex --dangerously-bypass-approvals-and-sandbox "Check tests code coverage! Increase to 100% tests code coverage!"
git add .
git commit -a -m"Tests" --author="Denis Stepulenok <Denis.Stepulenok@kuehne-nagel.com>"
git pull
git push origin 



