Reads a CSV file containing a list of a list of tags (metaexpressiions) and outputs two csv files containing the co-occurrence counts of each pair of tags.

Processes 900k lines per second, single threaded (takes around 40 seconds per GByte of data).

Run using

```
cargo run --release meta-expressions.csv
```

If you want to denoise the dataset by mapping emoticons to emojis and filtering out hashtags, run 

```
cargo run --release meta-expressions.csv -r replace_emoticons_and_ignore_hashtags
```


## Example


**input**

```csv
tweet_year,mex_num,emojis,emoticons,hashtags
2016,2,😞 😷,"",""
2016,2,"","",#p2 #pfla
2016,2,"","",#Diyala #Iraq
2016,1,"","",#JAChat
2016,1,💕,"",""
2016,1,"","",#MTVHOTTEST
2016,1,"","",#WTAE
2016,3,🆘 😏 😱,"",""
2016,1,❤️,"",""
2016,2,"","",#ALDUB55thWeeksary #NoToSkinnyJeansAlden
2016,3,"","",#DrinkLocalAR #IPAday #NWARK
2016,1,"","",#BarackObama
2016,2,❤️ 🎉,"",""
2016,2,"","",#9YearsWithGirlsGeneration #9YearsWithSNSD
2016,1,"","",#WavesJive
2016,3,🎈 👸🏽 💗,"",""
2016,1,❤️,"",""
2016,1,❤️,"",""
2016,2,🎉 😛,"",""
```

**output**

**==> nodes.csv <==**

```csv
node,weight
😂,39609599
:),26893870
😍,15693390
😭,14855483
❤️,11556696
:D,10714763
:(,9173239
😘,8550344
😊,8517846
😩,8499325
♥,8139142
❤,8023995
💕,7881915
;),6445429
😒,5870121
👌,5763942
😁,5186002
🔥,5151291
💯,5111518
...
```

**==> edges.csv <==**

```csv
node_1,node_2,weight
😂,😭,2629965
😍,😘,1469567
😂,😩,1335857
#iHeartAwards,#BestFanArmy,1092120
❤️,😍,865225
😩,😭,813709
😂,💀,801893
😭,😍,781185
❤,😍,731254
❤️,😘,645695
😂,😍,631402
💕,😍,618972
💙,💛,613675
🎉,🎈,606803
❤,😘,598166
🎉,🎊,595216
😩,😍,591078
❤️,😭,586766
💙,💚,569619
...
```